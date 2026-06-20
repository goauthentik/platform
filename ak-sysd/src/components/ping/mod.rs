use crate::components::Component;
use ak_meta::full_version;
use ak_platform::generated::ping::{CapabilitiesResponse, PingResponse, ping_server::Ping};
use tonic::{Request, Response, Status};

pub struct PingComponent {}

impl Component for PingComponent {
    fn new() -> ak_platform::prelude::Result<Box<dyn Component>> {
        Ok(Box::new(Self {}))
    }

    fn start(&self) -> ak_platform::prelude::Result<()> {
        Ok(())
    }

    fn stop(&self) -> ak_platform::prelude::Result<()> {
        Ok(())
    }

    fn register_for_id(&self, _id: ak_platform::paths::SysdSocketID) {
        todo!()
    }

    fn id() -> String {
        "ping".to_string()
    }
}

#[tonic::async_trait]
impl Ping for PingComponent {
    async fn ping(&self, _request: Request<()>) -> Result<Response<PingResponse>, Status> {
        Ok(Response::new(PingResponse {
            component: "sysd".to_string(),
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
