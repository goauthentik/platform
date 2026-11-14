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

/// Unified component lifecycle + gRPC-registration contract
///
/// `id`/`register` are `Self: Sized` — called once at construction time on
/// the concrete type, before it's erased into `Arc<dyn Component>` for
/// generic start/stop/restart iteration.
#[tonic::async_trait]
pub trait Component: Send + Sync {
    fn id() -> &'static str
    where
        Self: Sized;

    async fn start(&self) -> Result<()> {
        Ok(())
    }
    async fn stop(&self) -> Result<()> {
        Ok(())
    }

    /// Plugs this component's generated gRPC server(s) into the shared
    /// per-socket route builder.
    fn register(self: Arc<Self>, _socket: SysdSocketID, _routes: &mut tonic::service::RoutesBuilder)
    where
        Self: Sized,
    {
    }
}
