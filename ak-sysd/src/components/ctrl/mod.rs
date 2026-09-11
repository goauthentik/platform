use crate::components::{Component, SysdContext};
use crate::events::{ConfigChangeKind, SysdEvent};
use crate::state::TroubleshootNode;
use crate::util::to_status;
use ak_platform::generated::sys_ctrl::{
    Domain, DomainEnrollRequest, DomainEnrollResponse, DomainListResponse,
    TroubleshootInspectResponse,
    system_ctrl_server::{SystemCtrl, SystemCtrlServer},
};
use ak_platform::paths::SysdSocketID;
use eyre::Result;
use std::sync::Arc;
use tonic::{Request, Response, Status};

pub struct CtrlComponent {
    ctx: SysdContext,
}

impl CtrlComponent {
    pub fn new(ctx: SysdContext) -> CtrlComponent {
        CtrlComponent { ctx }
    }

    fn validate_domain_name(&self, name: &str) -> Result<(), Status> {
        let valid = !name.is_empty()
            && name
                .chars()
                .next()
                .is_some_and(|c| c.is_ascii_alphanumeric())
            && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '-');
        if !valid {
            return Err(Status::invalid_argument(
                "domain name must match ^[a-zA-Z0-9][a-zA-Z0-9-]*$",
            ));
        }
        Ok(())
    }

    fn node_to_response(&self, node: TroubleshootNode) -> TroubleshootInspectResponse {
        TroubleshootInspectResponse {
            bucket: node.bucket,
            kv: node.kv.into_iter().collect(),
            children: node
                .children
                .into_iter()
                .map(|n| self.node_to_response(n))
                .collect(),
        }
    }
}

#[tonic::async_trait]
impl Component for CtrlComponent {
    fn id() -> &'static str {
        "ctrl"
    }

    async fn start(&self) -> Result<()> {
        Ok(())
    }

    async fn stop(&self) -> Result<()> {
        Ok(())
    }

    fn register(self: Arc<Self>, socket: SysdSocketID, routes: &mut tonic::service::RoutesBuilder) {
        if matches!(socket, SysdSocketID::CTRL) {
            routes.add_service(SystemCtrlServer::from_arc(self));
        }
    }
}

#[tonic::async_trait]
impl SystemCtrl for CtrlComponent {
    async fn domain_list(
        &self,
        _request: Request<()>,
    ) -> Result<Response<DomainListResponse>, Status> {
        let domains = self
            .ctx
            .domains
            .domains()
            .await
            .into_iter()
            .map(|d| Domain {
                name: d.cfg.domain.clone(),
            })
            .collect();
        Ok(Response::new(DomainListResponse { domains }))
    }

    async fn domain_enroll(
        &self,
        request: Request<DomainEnrollRequest>,
    ) -> Result<Response<DomainEnrollResponse>, Status> {
        let req = request.into_inner();
        self.validate_domain_name(&req.name)?;

        let cfg = self
            .ctx
            .domains
            .enroll(req.name, req.authentik_url, req.token)
            .await
            .map_err(to_status)?;
        self.ctx.domains.test(&cfg).await.map_err(to_status)?;
        self.ctx
            .domains
            .save_domain(cfg.clone())
            .await
            .map_err(to_status)?;

        // ConfigChanged is dispatched fire-and-forget (watch_config_changes
        // processes it on a separate spawned task), so without this, callers
        // of domain_enroll (e.g. `ak-sysd domains join`) return before
        // `remote`/`brand` are populated, racing any immediate auth attempt
        // against interactive_auth/token_auth's `active.remote` reads.
        if let Err(e) = self.ctx.domains.fetch_remote_config(&cfg.domain).await {
            tracing::warn!(domain = cfg.domain, "post-enroll healthcheck failed: {e:?}");
        }

        self.ctx.events.dispatch(SysdEvent::ConfigChanged {
            kind: ConfigChangeKind::Added,
        });

        if let Some(device) = self
            .ctx
            .registry
            .get::<crate::components::device::DeviceComponent>("device")
            && let Err(e) = device.checkin_domain(&cfg.domain).await
        {
            tracing::warn!("post-enroll checkin failed: {e:?}");
        }

        // Go's response never actually sets device_id (`&pb.DomainEnrollResponse{}`) —
        // porting that literal (likely latent-gap) behavior rather than fixing it silently.
        Ok(Response::new(DomainEnrollResponse {
            device_id: String::new(),
        }))
    }

    async fn domain_unenroll(&self, request: Request<Domain>) -> Result<Response<()>, Status> {
        let req = request.into_inner();
        let exists = self
            .ctx
            .domains
            .domains()
            .await
            .iter()
            .any(|d| d.cfg.domain == req.name);
        if !exists {
            return Err(Status::not_found("domain not found"));
        }
        self.ctx
            .domains
            .delete_domain(&req.name)
            .await
            .map_err(to_status)?;
        Ok(Response::new(()))
    }

    async fn troubleshoot_inspect(
        &self,
        _request: Request<()>,
    ) -> Result<Response<TroubleshootInspectResponse>, Status> {
        let node = self.ctx.state.inspect().await.map_err(to_status)?;
        Ok(Response::new(self.node_to_response(node)))
    }
}

#[cfg(test)]
mod test {
    use ak_platform::generated::sys_ctrl::system_ctrl_server::SystemCtrl;
    use tonic::Request;

    use crate::{components::ctrl::CtrlComponent, context::testutils::test_context};

    #[tokio::test]
    async fn test_inspect() {
        let ctrl = CtrlComponent::new(test_context().await);
        let res = ctrl
            .troubleshoot_inspect(Request::new(()))
            .await
            .unwrap()
            .into_inner();
        assert_eq!(res.bucket, "root");
    }
}
