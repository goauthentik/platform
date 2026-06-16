use crate::agent::Agent;
use ak_platform::{log, prelude::*, string::PlatformString};

pub mod agent;
pub mod config;
pub mod grpc;
pub mod ssh;
pub mod token;

#[tokio::main]
async fn main() -> Result<()> {
    log::init_log(
        PlatformString::new()
            .with_windows("authentik User Service")
            .with_linux("ak-agent"),
    );
    ak_platform_keyring::init()?;
    let ag = Agent::new().await?;
    #[cfg(unix)]
    {
        use tokio::signal::unix::{SignalKind, signal};
        let mut sigterm = signal(SignalKind::terminate())?;
        tokio::select! {
            result = ag.start() => result?,
            _ = sigterm.recv() => {}
        }
    }
    #[cfg(not(unix))]
    ag.start().await?;
    Ok(())
}
