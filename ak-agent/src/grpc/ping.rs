use ak_meta::full_version;
use ak_platform::generated::ping::{CapabilitiesResponse, PingResponse, ping_server::Ping};
use authentik_client::apis::admin_api::admin_version_retrieve;
use tonic::{Request, Response, Status};

use crate::grpc::AgentGRPCServer;

#[tonic::async_trait]
impl Ping for AgentGRPCServer {
    async fn ping(&self, _request: Request<()>) -> Result<Response<PingResponse>, Status> {
        let prof = self.agent.cfg.read().await.active_profile.clone();
        let api = self
            .agent
            .cfg
            .read()
            .await
            .profiles
            .get(&prof)
            .ok_or_else(|| Status::not_found("profile not found"))?
            .clone()
            .api_config()
            .map_err(Status::from_error)?;
        let ver = admin_version_retrieve(&api)
            .await
            .map_err(|e| Status::from_error(e.into()))?;
        Ok(Response::new(PingResponse {
            component: "agent".to_string(),
            version: full_version(),
            server_version: ver.version_current,
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
