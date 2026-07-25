use crate::components::{Component, SysdContext};
use ak_meta::full_version;
use ak_platform::generated::ping::{
    CapabilitiesResponse, PingResponse,
    capabilities_response::Capability,
    ping_server::{Ping, PingServer},
};
use ak_platform::paths::SysdSocketID;
use authentik_client::models::LicenseStatusEnum;
use std::sync::Arc;
use tonic::{Request, Response, Status};

pub struct PingComponent {
    ctx: SysdContext,
}

impl PingComponent {
    pub fn new(ctx: SysdContext) -> PingComponent {
        PingComponent { ctx }
    }
}

#[tonic::async_trait]
impl Component for PingComponent {
    fn id() -> &'static str {
        "ping"
    }

    async fn start(&self) -> eyre::Result<()> {
        Ok(())
    }

    async fn stop(&self) -> eyre::Result<()> {
        Ok(())
    }

    fn register(self: Arc<Self>, socket: SysdSocketID, routes: &mut tonic::service::RoutesBuilder) {
        if matches!(socket, SysdSocketID::Default) {
            routes.add_service(PingServer::from_arc(self));
        }
    }
}

#[tonic::async_trait]
impl Ping for PingComponent {
    async fn ping(&self, _request: Request<()>) -> Result<Response<PingResponse>, Status> {
        Ok(Response::new(PingResponse {
            component: "sysd".to_string(),
            version: full_version(),
            server_version: "".to_string(),
        }))
    }

    async fn capabilities(
        &self,
        _request: Request<()>,
    ) -> Result<Response<CapabilitiesResponse>, Status> {
        let mut capabilities = vec![];

        if self.ctx.cfg.read().await.debug {
            capabilities.push(Capability::Debug as i32);
        }

        // Mirrors Go's `InteractiveSupported()` (license status != UNLICENSED).
        if let Ok(active) = self.ctx.domains.active().await
            && let Some(remote) = active.remote.read().await.as_ref()
            && let Some(status) = remote.license_status
            && status != LicenseStatusEnum::Unlicensed
        {
            capabilities.push(Capability::AuthInteractive as i32);
        }

        Ok(Response::new(CapabilitiesResponse { capabilities }))
    }
}
