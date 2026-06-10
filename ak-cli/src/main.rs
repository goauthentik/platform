use ak_platform::{
    client::user::{AnyService, Client},
    log::set_log_level,
};
use clap::{Error, Parser, Subcommand};
use log::LevelFilter;

use crate::commands::{auth::AuthCommands, config::ConfigCommands};

pub mod auth;
pub mod cache;
pub mod commands;
pub mod format;
pub mod setup;

#[derive(Parser, Clone)]
#[command(name = "authentik CLI")]
#[command(version, about, long_about = None)]
#[command(propagate_version = true)]
pub struct CliArgs {
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

#[derive(Clone)]
pub struct App {
    args: CliArgs,
    client: Option<Client<AnyService>>,
}

#[derive(Subcommand, Clone)]
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

impl App {
    pub async fn user(mut self) -> Result<Client<AnyService>, Box<dyn std::error::Error>> {
        match self.client {
            Some(c) => Ok(c),
            None => {
                let c = Client::new().await?;
                self.client = Some(c.clone());
                Ok(c)
            }
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    let cli = CliArgs::parse();

    set_log_level(LevelFilter::Warn);
    if cli.verbose {
        set_log_level(LevelFilter::Trace);
    }

    let app  = App {
        args: cli.clone(),
        client: None,
    };

    let res = match &cli.command {
        Commands::Whoami => commands::whoami::whoami(app).await,
        Commands::Version => commands::version::version(app).await,
        Commands::Config { command } => match command {
            ConfigCommands::ListProfiles => commands::config::list_profiles(app).await,
            ConfigCommands::Setup {
                authentik_url,
                client_id,
                app_slug,
            } => commands::config::setup(app, authentik_url, client_id, app_slug).await,
        },
        Commands::Auth { command } => match command {
            AuthCommands::Raw { client_id } => commands::auth::raw(app, client_id).await,
            AuthCommands::Kubectl { client_id } => commands::auth::kubectl(app, client_id).await,
            AuthCommands::Aws {
                client_id,
                role_arn,
                region,
            } => commands::auth::aws(app, client_id, role_arn, region).await,
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
