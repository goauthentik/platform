use ak_platform::generated::{
    agent::{RequestHeader, ResponseHeader},
    agent_ctrl::{
        CurrentProfileResponse, ListProfilesResponse, Profile, ProfileStatus, SetupRequest,
        SetupResponse, agent_ctrl_server::AgentCtrl,
    },
};
use tonic::{Request, Response, Status};

use crate::config::ConfigV1Profile;
use crate::grpc::AgentGRPCServer;

#[tonic::async_trait]
impl AgentCtrl for AgentGRPCServer {
    async fn list_profiles(
        &self,
        _request: Request<()>,
    ) -> Result<Response<ListProfilesResponse>, Status> {
        let mut profiles = vec![];
        for (key, c_prof) in self.agent.cfg.read().await.profiles.iter() {
            let failed_profile = || Profile {
                name: key.clone(),
                username: String::new(),
                authentik_url: c_prof.authentik_url.clone(),
                last_renewed: None,
                next_renew: None,
                status: ProfileStatus::Failed as i32,
            };

            let Some(ptm) = self.agent.gtm.for_profile(key).await else {
                let reason = self.agent.gtm.creation_failure(key).await;
                tracing::warn!(profile = key, "no token manager for profile: {reason:?}");
                profiles.push(failed_profile());
                continue;
            };

            let token = match ptm.unverified().await {
                Ok(t) => t,
                Err(e) => {
                    tracing::warn!(profile = key, "failed to read token: {e:?}");
                    profiles.push(failed_profile());
                    continue;
                }
            };
            let claims = match token.claims() {
                Ok(c) => c,
                Err(e) => {
                    tracing::warn!(profile = key, "failed to parse claims: {e:?}");
                    profiles.push(failed_profile());
                    continue;
                }
            };

            let status = match ptm.is_failed().await {
                Some(e) => {
                    tracing::warn!(profile = key, "last renewal failed: {e}");
                    ProfileStatus::Failed
                }
                None => ProfileStatus::Active,
            };

            profiles.push(Profile {
                name: key.clone(),
                username: claims.preferred_username,
                authentik_url: c_prof.authentik_url.clone(),
                last_renewed: Some(claims.iat.into()),
                next_renew: Some(claims.exp.into()),
                status: status as i32,
            });
        }
        Ok(Response::new(ListProfilesResponse {
            header: Some(ResponseHeader { successful: true }),
            profiles,
        }))
    }

    async fn setup(
        &self,
        request: Request<SetupRequest>,
    ) -> Result<Response<SetupResponse>, Status> {
        let req = request.into_inner();
        let profile_name = req
            .header
            .ok_or(Status::invalid_argument("missing header"))?
            .profile;
        {
            let mut cfg = self.agent.cfg.write().await;
            cfg.profiles.insert(
                profile_name.clone(),
                ConfigV1Profile::from_tokens(
                    req.authentik_url,
                    req.app_slug,
                    req.client_id,
                    req.access_token,
                    req.refresh_token,
                ),
            );
            if cfg.active_profile.is_empty() {
                cfg.active_profile = profile_name.clone();
            }
        }
        if let Err(e) = self.agent.cfg.save().await {
            tracing::warn!("failed to save config: {e:?}");
            return Err(Status::from_error(e.into()));
        }
        self.agent.gtm.wait_for_profile(&profile_name).await;
        tracing::info!(profile = profile_name, "setup new profile");
        Ok(Response::new(SetupResponse {
            header: Some(ResponseHeader { successful: true }),
        }))
    }

    async fn switch_profile(
        &self,
        request: Request<RequestHeader>,
    ) -> Result<Response<ResponseHeader>, Status> {
        let new_profile = request.into_inner().profile;
        {
            let mut cfg = self.agent.cfg.write().await;
            cfg.active_profile = new_profile.clone();
        }
        if let Err(e) = self.agent.cfg.save().await {
            tracing::warn!("failed to save config: {e:?}");
            return Err(Status::from_error(e.into()));
        }
        tracing::debug!(profile = new_profile, "Switched active profile");
        Ok(Response::new(ResponseHeader { successful: true }))
    }

    async fn current_profile(
        &self,
        _request: Request<()>,
    ) -> Result<Response<CurrentProfileResponse>, Status> {
        let cfg = self.agent.cfg.read().await;
        Ok(Response::new(CurrentProfileResponse {
            header: Some(ResponseHeader { successful: true }),
            profile: cfg.active_profile.clone(),
        }))
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use ak_platform::shared::AuthentikClaims;
    use ak_platform::storage::cfgmgr::testutils::test_config_manager;
    use chrono::{TimeDelta, Utc};
    use jsonwebtoken::{Algorithm, EncodingKey, Header, encode};

    use super::*;
    use crate::Agent;
    use crate::config::ConfigV1;
    use crate::token::global::GlobalTokenManager;
    use crate::token::testutils::{gtm_test_lock, spawn_http_once};

    fn fake_access_token(username: &str) -> String {
        let claims = AuthentikClaims {
            iss: "https://authentik.example".to_string(),
            sub: None,
            aud: vec!["client".to_string()],
            exp: Utc::now() + TimeDelta::hours(1),
            iat: Utc::now(),
            jti: None,
            preferred_username: username.to_string(),
        };
        encode(
            &Header::new(Algorithm::HS256),
            &claims,
            &EncodingKey::from_secret(b"test"),
        )
        .unwrap()
    }

    #[tokio::test]
    async fn list_profiles_reports_failed_profile_without_aborting() {
        let _guard = gtm_test_lock().lock().unwrap_or_else(|e| e.into_inner());

        let jwks_url = spawn_http_once("HTTP/1.1 200 OK", r#"{"keys":[]}"#).await;

        let cfg = test_config_manager::<ConfigV1>();
        cfg.write().await.profiles.insert(
            "good".to_string(),
            ConfigV1Profile::from_tokens(
                jwks_url,
                "app".to_string(),
                "client".to_string(),
                fake_access_token("alice"),
                "refresh".to_string(),
            ),
        );

        let gtm = GlobalTokenManager::new(Arc::clone(&cfg)).await.unwrap();

        // Added *after* the manager was built, bypassing new_verified
        // entirely: reproduces "profile in config with no live manager" the
        // same way a failed JWKS fetch at startup would, without needing a
        // second mock server that deliberately refuses the connection.
        cfg.write().await.profiles.insert(
            "bad".to_string(),
            ConfigV1Profile::from_tokens(
                "http://127.0.0.1:1".to_string(),
                "app".to_string(),
                "client".to_string(),
                "access".to_string(),
                "refresh".to_string(),
            ),
        );

        let agent = Arc::new(Agent {
            cfg,
            gtm: Arc::new(gtm),
        });
        let server = AgentGRPCServer::new(agent).await.unwrap();

        let resp = server
            .list_profiles(Request::new(()))
            .await
            .unwrap()
            .into_inner();

        assert_eq!(resp.profiles.len(), 2);

        let good = resp.profiles.iter().find(|p| p.name == "good").unwrap();
        assert_eq!(good.status, ProfileStatus::Active as i32);
        assert_eq!(good.username, "alice");

        let bad = resp.profiles.iter().find(|p| p.name == "bad").unwrap();
        assert_eq!(bad.status, ProfileStatus::Failed as i32);
    }
}
