use ak_platform::generated::{
    agent::{RequestHeader, ResponseHeader},
    agent_ctrl::{
        CurrentProfileResponse, ListProfilesResponse, Profile, SetupRequest, SetupResponse,
        agent_ctrl_server::AgentCtrl,
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
            let ptm = self
                .agent
                .gtm
                .for_profile(key)
                .await
                .ok_or(Status::invalid_argument("profile not found"))?;
            let token = ptm.unverified().await.map_err(|e| Status::from_error(e.into()))?;
            let claims = token.claims().map_err(|e| Status::from_error(e.into()))?;
            let o_prof = Profile {
                name: key.clone(),
                username: claims.preferred_username,
                authentik_url: c_prof.authentik_url.clone(),
                last_renewed: Some(claims.iat.into()),
                next_renew: Some(claims.exp.into()),
            };
            profiles.push(o_prof);
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
