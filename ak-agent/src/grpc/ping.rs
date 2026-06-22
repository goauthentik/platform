use ak_meta::full_version;
use ak_platform::generated::ping::{CapabilitiesResponse, PingResponse, ping_server::Ping};
use tonic::{Request, Response, Status};

use crate::grpc::AgentGRPCServer;

#[tonic::async_trait]
impl Ping for AgentGRPCServer {
    async fn ping(&self, _request: Request<()>) -> Result<Response<PingResponse>, Status> {
        Ok(Response::new(PingResponse {
            component: "agent".to_string(),
            version: full_version(),
        }))
    }

    async fn capabilities(
        &self,
        _request: Request<()>,
    ) -> Result<Response<CapabilitiesResponse>, Status> {
        Ok(Response::new(CapabilitiesResponse {
            capabilities: vec![],
        }))
    }
}
