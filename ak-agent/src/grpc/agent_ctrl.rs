use ak_platform::generated::{
    agent::ResponseHeader,
    agent_ctrl::{
        ListProfilesResponse, Profile, SetupRequest, SetupResponse, agent_ctrl_server::AgentCtrl,
    },
};
use tonic::{Request, Response, Status};

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
        _request: Request<SetupRequest>,
    ) -> Result<Response<SetupResponse>, Status> {
        todo!()
    }
}
