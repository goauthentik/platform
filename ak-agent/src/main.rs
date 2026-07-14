use crate::agent::Agent;
use ak_meta::full_version;
use ak_platform::string::PlatformString;
use eyre::Result;

pub mod agent;
pub mod config;
pub mod grpc;
pub mod ssh;
pub mod token;

#[ak_meta::main("ak-agent")]
async fn main() -> Result<()> {
    ak_platform::log::init_log(
        PlatformString::new()
            .with_windows("authentik User Service")
            .with_linux("ak-agent"),
    );
    tracing::trace!("authentik Agent v{}", full_version());
    ak_platform_keyring::init().map_err(|e| eyre::eyre!("{e}"))?;
    let ag = Agent::new().await?;
    ag.start().await?;
    Ok(())
}
