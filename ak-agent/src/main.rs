use ak_platform::log::init_log_interactive;
use ak_platform::{keyring, prelude::*};

use crate::agent::Agent;

pub mod agent;
pub mod config;
pub mod grpc;
pub mod ssh;
pub mod token;

#[tokio::main]
async fn main() -> Result<()> {
    keyring::init()?;
    init_log_interactive();
    let ag = Agent::new().await?;
    ag.start().await?;
    Ok(())
}
