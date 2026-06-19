use ak_platform::paths::sysd_config_file;
use clap::{Error, Parser, Subcommand};


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
pub async fn main ()  {

}
