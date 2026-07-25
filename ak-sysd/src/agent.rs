use crate::cfg::domain::DomainManager;
use crate::components::Component;
use crate::components::agent_starter::AgentStarterComponent;
use crate::components::auth::AuthComponent;
use crate::components::ctrl::CtrlComponent;
use crate::components::device::DeviceComponent;
use crate::components::ping::PingComponent;
use crate::context::SysdContext;
use crate::events::{ConfigChangeKind, SysdEvent};
use crate::state::StateStore;
use ak_platform::net::server::{SocketPermMode, listen};
use ak_platform::paths::{SysdSocketID, sysd_socket_path};
use ak_platform::storage::cfgmgr::ConfigManager;
use eyre::Result;
use sentry_tower::{NewSentryLayer, SentryHttpLayer};
use std::sync::Arc;
use tonic::service::RoutesBuilder;
use tonic::transport::Server;
use tower_http::trace::{DefaultOnFailure, DefaultOnRequest, TraceLayer};
use tracing::Level;

#[cfg(target_os = "linux")]
use crate::components::directory::DirectoryComponent;
#[cfg(any(target_os = "linux", target_os = "windows"))]
use crate::components::session::SessionComponent;

pub struct Agent {
    ctx: SysdContext,
    components: Vec<Arc<dyn Component>>,
    default_routes: RoutesBuilder,
    ctrl_routes: RoutesBuilder,
}

impl Agent {
    pub async fn new(config_path: String) -> Result<Self> {
        let cfg = ConfigManager::<crate::cfg::Config>::new(config_path).await?;
        let (runtime_dir, domain_dir) = {
            let read = cfg.read().await;
            (read.runtime_dir.clone(), read.domain_dir.clone())
        };

        let state = Arc::new(StateStore::open(&format!("{runtime_dir}/sysd-state.db"))?);
        let domains = DomainManager::new(domain_dir, Arc::clone(&state)).await?;
        domains.load_all().await?;

        let ctx = SysdContext::new(cfg, domains, state)?;

        let mut default_routes = RoutesBuilder::default();
        let mut ctrl_routes = RoutesBuilder::default();
        let components =
            Self::register_platform_components(&ctx, &mut default_routes, &mut ctrl_routes);

        let ag = Agent {
            ctx,
            components,
            default_routes,
            ctrl_routes,
        };
        ag.watch_config_changes();
        Ok(ag)
    }

    /// Constructs and registers every component for the current platform,
    /// mirroring Go's per-GOOS lists in `pkg/agent_system/components_{darwin,linux,windows}.go`:
    /// darwin has no directory/session, windows has no directory, linux has everything.
    fn register_platform_components(
        ctx: &SysdContext,
        default_routes: &mut RoutesBuilder,
        ctrl_routes: &mut RoutesBuilder,
    ) -> Vec<Arc<dyn Component>> {
        let mut components: Vec<Arc<dyn Component>> = vec![];

        macro_rules! register {
            ($ty:ty) => {{
                let comp = Arc::new(<$ty>::new(ctx.clone()));
                ctx.registry.insert(<$ty>::id(), Arc::clone(&comp));
                Arc::clone(&comp).register(SysdSocketID::Default, default_routes);
                Arc::clone(&comp).register(SysdSocketID::CTRL, ctrl_routes);
                components.push(comp as Arc<dyn Component>);
            }};
        }

        register!(AgentStarterComponent);
        register!(AuthComponent);
        register!(DeviceComponent);
        register!(PingComponent);
        #[cfg(target_os = "linux")]
        register!(DirectoryComponent);
        #[cfg(any(target_os = "linux", target_os = "windows"))]
        register!(SessionComponent);
        register!(CtrlComponent);

        components
    }

    fn watch_config_changes(&self) {
        let ctx = self.ctx.clone();
        let components = self.components.clone();
        let mut rx = ctx.events.subscribe();
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    ev = rx.recv() => {
                        match ev {
                            Ok(SysdEvent::ConfigChanged {
                                kind: ConfigChangeKind::Added | ConfigChangeKind::Removed,
                            }) => {
                                tracing::info!("domain config changed, restarting components");
                                for c in &components {
                                    if let Err(e) = c.stop().await {
                                        tracing::warn!("component failed to stop: {e:?}");
                                    }
                                }
                                if let Err(e) = ctx.domains.load_all().await {
                                    tracing::warn!("failed to reload domains: {e:?}");
                                }
                                for c in &components {
                                    if let Err(e) = c.start().await {
                                        tracing::warn!("component failed to start: {e:?}");
                                    }
                                }
                            }
                            Ok(_) => {}
                            Err(_) => return,
                        }
                    }
                    () = ctx.cancel.cancelled() => return,
                }
            }
        });
    }

    fn serve(&self, socket: SysdSocketID, perm: SocketPermMode, routes: RoutesBuilder) {
        let cancel = self.ctx.cancel.clone();
        tokio::spawn(async move {
            let listener = match listen(sysd_socket_path(socket), perm).await {
                Ok(l) => l,
                Err(e) => {
                    tracing::error!("failed to listen on socket: {e:?}");
                    return;
                }
            };
            let result = Server::builder()
                .layer(NewSentryLayer::new_from_top())
                .layer(SentryHttpLayer::new().enable_transaction())
                .layer(
                    TraceLayer::new_for_grpc()
                        .on_request(DefaultOnRequest::new().level(Level::INFO))
                        .on_failure(DefaultOnFailure::new().level(Level::ERROR)),
                )
                .add_routes(routes.routes())
                .serve_with_incoming_shutdown(listener, cancel.cancelled_owned())
                .await;
            if let Err(e) = result {
                tracing::error!("socket server exited: {e:?}");
            }
        });
    }

    pub async fn start(&self) -> Result<()> {
        for c in &self.components {
            if let Err(e) = c.start().await {
                tracing::warn!("component failed to start: {e:?}");
            }
        }
        self.ctx.domains.healthcheck_all().await;
        self.ctx.events.dispatch(SysdEvent::LifecycleStarted);

        self.serve(
            SysdSocketID::Default,
            SocketPermMode::Everyone,
            self.default_routes.clone(),
        );
        self.serve(
            SysdSocketID::CTRL,
            SocketPermMode::Admin,
            self.ctrl_routes.clone(),
        );

        Ok(())
    }

    pub async fn wait(&self) -> Result<()> {
        #[cfg(unix)]
        {
            let mut sigterm =
                tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())?;
            tokio::select! {
                _ = tokio::signal::ctrl_c() => {},
                _ = sigterm.recv() => {},
            }
        }
        #[cfg(not(unix))]
        {
            tokio::signal::ctrl_c().await.ok();
        }
        self.stop().await
    }

    pub async fn stop(&self) -> Result<()> {
        self.ctx.cancel.cancel();
        for c in &self.components {
            if let Err(e) = c.stop().await {
                tracing::warn!("component failed to stop: {e:?}");
            }
        }
        Ok(())
    }
}
