use ak_platform::paths::sysd_config_file;
use clap::{Error, Parser, Subcommand};
use ak_platform::prelude::*;
use crate::agent::Agent;

pub mod agent;
pub mod cfg;
pub mod components;

#[derive(Parser, Clone)]
#[command(name = "authentik System Daemon")]
#[command(version, about, long_about = None)]
#[command(propagate_version = true)]
pub struct SysdArgs {
    /// Config file path
    #[arg(short, long, default_value_t = sysd_config_file().for_current())]
    config: String,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Clone)]
enum Commands {
    /// Run the authentik system agent
    Agent,
}

#[tokio::main]
pub async fn main() -> Result<()> {
    let cli = SysdArgs::parse();

    match &cli.command {
        Commands::Agent => {
            let ag = Agent::new(cli.config).await?;
            ag.start().await?;
        }
    }
    Ok(())
}
