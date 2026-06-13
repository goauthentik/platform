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
        let profiles = self
            .agent
            .cfg
            .read()
            .await
            .profiles
            .keys()
            .map(|k| Profile { name: k.clone() })
            .collect::<Vec<Profile>>();
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
                profile_name,
                ConfigV1Profile::from_tokens(
                    req.authentik_url,
                    req.app_slug,
                    req.client_id,
                    req.access_token,
                    req.refresh_token,
                ),
            );
        }
        self.agent.cfg.save().await.map_err(Status::from_error)?;
        Ok(Response::new(SetupResponse {
            header: Some(ResponseHeader { successful: true }),
        }))
    }
}
