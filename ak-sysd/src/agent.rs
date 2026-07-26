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
use std::collections::HashMap;
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
    components: HashMap<String, Arc<dyn Component>>,
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
    ) -> HashMap<String, Arc<dyn Component>> {
        let mut components: HashMap<String, Arc<dyn Component>> = HashMap::new();

        macro_rules! register {
            ($ty:ty) => {{
                let comp = Arc::new(<$ty>::new(ctx.clone()));
                ctx.registry.insert(<$ty>::id(), Arc::clone(&comp));
                Arc::clone(&comp).register(SysdSocketID::Default, default_routes);
                Arc::clone(&comp).register(SysdSocketID::CTRL, ctrl_routes);

                components.insert(<$ty>::id().to_string(), comp as Arc<dyn Component>);
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
                                for (id, c) in &components {
                                    tracing::info!(component = id, "stopping component");
                                    if let Err(e) = c.stop().await {
                                        tracing::warn!("component failed to stop: {e:?}");
                                    }
                                }
                                if let Err(e) = ctx.domains.load_all().await {
                                    tracing::warn!("failed to reload domains: {e:?}");
                                }
                                for (id, c) in &components {
                                    tracing::info!(component = id, "starting component");
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

    /// Arms an out-of-runtime backstop so that a *second* SIGINT/SIGTERM
    /// force-terminates the process immediately, even if the tokio runtime is
    /// wedged
    #[cfg(unix)]
    fn install_force_exit_backstop() -> Result<()> {
        use signal_hook::consts::{SIGINT, SIGTERM};
        let forced = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        for sig in [SIGINT, SIGTERM] {
            // Order matters: the conditional shutdown must be registered before
            // the flag setter so the first delivery only arms the flag and the
            // second delivery triggers the immediate exit.
            signal_hook::flag::register_conditional_shutdown(
                sig,
                130,
                std::sync::Arc::clone(&forced),
            )?;
            signal_hook::flag::register(sig, std::sync::Arc::clone(&forced))?;
        }
        Ok(())
    }

    pub async fn start(&self) -> Result<()> {
        // Arm the force-exit backstop before any component (and its blocking
        // subprocess calls) can wedge the runtime.
        #[cfg(unix)]
        Self::install_force_exit_backstop()?;

        for (id, c) in &self.components {
            tracing::info!(component = id, "starting component");
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
        // Graceful path (first signal): handled by the async runtime when healthy.
        // The out-of-runtime force-exit backstop for the *second* signal is armed
        // earlier, in `start()`, so it survives a runtime wedge that happens before
        // we reach this point.
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

        tracing::info!("shutdown signal received; stopping (send the signal again to force quit)");

        // Bound graceful shutdown so a hung component can't wedge exit forever.
        match tokio::time::timeout(std::time::Duration::from_secs(15), self.stop()).await {
            Ok(res) => res,
            Err(_) => {
                tracing::warn!("graceful shutdown timed out after 15s, forcing exit");
                std::process::exit(130);
            }
        }
    }

    pub async fn stop(&self) -> Result<()> {
        self.ctx.cancel.cancel();
        for (id, c) in &self.components {
            tracing::info!(component = id, "stopping component");
            if let Err(e) = c.stop().await {
                tracing::warn!("component failed to stop: {e:?}");
            }
        }
        Ok(())
    }
}
