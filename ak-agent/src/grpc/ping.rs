use ak_meta::full_version;
use ak_platform::generated::ping::{CapabilitiesResponse, PingResponse, ping_server::Ping};
use authentik_client::{apis::admin_api::admin_version_retrieve, models::Version};
use tonic::{Request, Response, Status};

use crate::grpc::AgentGRPCServer;

impl AgentGRPCServer {
    async fn server_version(&self) -> Result<Version, Status> {
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
            .map_err(|e| Status::from_error(e.into()))?;
        let ver = admin_version_retrieve(&api)
            .await
            .map_err(|e| Status::from_error(e.into()))?;
        Ok(ver)
    }
}

#[tonic::async_trait]
impl Ping for AgentGRPCServer {
    async fn ping(&self, _request: Request<()>) -> Result<Response<PingResponse>, Status> {
        let mut res = PingResponse {
            component: "agent".to_string(),
            version: full_version(),
            server_version: "".to_string(),
        };
        match self.server_version().await {
            Ok(v) => res.server_version = v.version_current,
            Err(e) => {
                tracing::warn!("Failed to get server version: {e:?}");
            }
        };
        Ok(Response::new(res))
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
