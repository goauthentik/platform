use ak_platform::generated::{
    agent::ResponseHeader,
    agent_ctrl::{
        ListProfilesResponse, Profile, SetupRequest, SetupResponse, agent_ctrl_server::AgentCtrl,
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
            let token = ptm.token().await.map_err(Status::from_error)?;
            let claims = token.claims().map_err(Status::from_error)?;
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
        }
        if let Err(e) = self.agent.cfg.save().await {
            log::warn!("failed to save config: {e:?}");
            return Err(Status::from_error(e));
        }
        self.agent.gtm.wait_for_profile(&profile_name).await;
        log::info!("setup new profile {profile_name}");
        Ok(Response::new(SetupResponse {
            header: Some(ResponseHeader { successful: true }),
        }))
    }
}
