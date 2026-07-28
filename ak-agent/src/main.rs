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
    ak_platform::log::LogBuilder::new(
        PlatformString::new()
            .with_windows("authentik User Service")
            .with_linux("ak-agent"),
    )
    .default_level(ak_platform::log::LevelFilter::Info)
    .with_filter("ak_agent", ak_platform::log::LevelFilter::Trace)
    .enable();
    tracing::trace!("authentik Agent v{}", full_version());
    let ag = Agent::new().await?;
    ag.start().await?;
    Ok(())
}
