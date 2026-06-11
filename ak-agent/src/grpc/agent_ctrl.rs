use ak_platform::generated::agent_ctrl::{
    ListProfilesResponse, SetupRequest, SetupResponse, agent_ctrl_server::AgentCtrl,
};
use tonic::{Request, Response, Status};

use crate::grpc::AgentGRPCServer;

#[tonic::async_trait]
impl AgentCtrl for AgentGRPCServer {
    async fn list_profiles(
        &self,
        _request: Request<()>,
    ) -> Result<Response<ListProfilesResponse>, Status> {
        todo!()
    }
    async fn setup(
        &self,
        _request: Request<SetupRequest>,
    ) -> Result<Response<SetupResponse>, Status> {
        todo!()
    }
}
