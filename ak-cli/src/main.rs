use akp_logger::set_log_level;
use clap::{Error, Parser, Subcommand};
use log::LevelFilter;

use crate::commands::{auth::AuthCommands, config::ConfigCommands};

pub mod auth;
pub mod cache;
pub mod commands;
pub mod format;

#[derive(Parser)]
#[command(name = "authentik CLI")]
#[command(version, about, long_about = None)]
#[command(propagate_version = true)]
pub struct Cli {
    /// Enable debug logging
    #[arg(short, long, default_value_t = false)]
    verbose: bool,
    /// Output JSON data
    #[arg(short, long, default_value_t = false)]
    json: bool,
    /// A name for the profile
    #[arg(short, long, default_value = "default")]
    profile: String,
    /// Socket the agent is listening on
    #[arg(short, long)]
    socket: Option<String>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Check user account details for a given profile
    Whoami,
    /// Version of authentik Agent components
    Version,

    /// Configure authentik CLI
    Config {
        #[command(subcommand)]
        command: ConfigCommands,
    },
    /// Commands for authenticating with different CLI applications.
    Auth {
        #[command(subcommand)]
        command: AuthCommands,
    },
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    let cli = Cli::parse();

    set_log_level(LevelFilter::Warn);
    if cli.verbose {
        set_log_level(LevelFilter::Trace);
    }

    let res = match &cli.command {
        Commands::Whoami => commands::whoami::whoami(&cli).await,
        Commands::Version => commands::version::version(&cli).await,
        Commands::Config { command } => match command {
            ConfigCommands::ListProfiles => commands::config::list_profiles(&cli).await,
        },
        Commands::Auth { command } => match command {
            AuthCommands::Raw { client_id } => commands::auth::raw(&cli, client_id).await,
            AuthCommands::Aws {
                client_id,
                role_arn,
                region,
            } => commands::auth::aws(&cli, client_id, role_arn, region).await,
        },
    };
    match res {
        Ok(_) => Ok(()),
        Err(e) => {
            eprintln!("Error running command: {e:?}");
            Ok(())
        }
    }
}
