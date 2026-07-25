use ak_platform::paths::SysdSocketID;
use eyre::Result;
use std::sync::Arc;

pub use crate::context::{ComponentRegistry, SysdContext};

pub mod agent_starter;
pub mod auth;
pub mod ctrl;
pub mod device;
pub mod directory;
pub mod ping;
pub mod session;

/// Unified component lifecycle + gRPC-registration contract, mirroring Go's
/// `component.Component` interface (`pkg/agent_system/component/component.go`).
///
/// `id`/`register` are `Self: Sized` — called once at construction time on
/// the concrete type, before it's erased into `Arc<dyn Component>` for
/// generic start/stop/restart iteration.
#[tonic::async_trait]
pub trait Component: Send + Sync {
    fn id() -> &'static str
    where
        Self: Sized;

    async fn start(&self) -> Result<()>;
    async fn stop(&self) -> Result<()>;

    /// Plugs this component's generated gRPC server(s) into the shared
    /// per-socket route builder. Components with no gRPC surface (e.g.
    /// `agent_starter`) leave this empty.
    fn register(self: Arc<Self>, socket: SysdSocketID, routes: &mut tonic::service::RoutesBuilder)
    where
        Self: Sized;
}
