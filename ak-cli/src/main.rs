use crate::commands::{auth::AuthCommands, config::ConfigCommands};
use ak_platform::log::LevelFilter;
use ak_platform::paths::DEFAULT_PROFILE;
use ak_platform::prelude::*;
use ak_platform::{
    client::user::{AnyService, Client},
    log::{init_log_interactive, set_log_level},
};
use clap::{Error, Parser, Subcommand};
use clap_complete::Shell;

pub mod api;
pub mod auth;
pub mod cache;
pub mod commands;
pub mod format;
pub mod setup;

#[derive(Parser, Clone)]
#[command(name = "authentik CLI")]
#[command(about, long_about = None)]
pub struct CliArgs {
    /// Enable debug logging
    #[arg(short, long, default_value_t = false)]
    verbose: bool,
    /// Output JSON data
    #[arg(short, long, default_value_t = false)]
    json: bool,
    /// A name for the profile
    #[arg(short, long, default_value = DEFAULT_PROFILE)]
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
    /// Generate shell completion scripts
    Completion {
        /// Shell to generate completions for
        shell: Shell,
    },
    /// Directly interact with the authentik API
    Api {
        #[command(subcommand)]
        command: api::ApiCommand,
    },
}

impl App {
    pub async fn user(mut self) -> Result<Client<AnyService>> {
        match self.client {
            Some(c) => Ok(c),
            None => {
                let c = Client::new(self.args.socket).await?;
                self.client = Some(c.clone());
                Ok(c)
            }
        }
    }
}

#[tokio::main]
async fn main() -> std::result::Result<(), Error> {
    let cli = CliArgs::parse();

    init_log_interactive();
    set_log_level(LevelFilter::Warn);
    if cli.verbose {
        set_log_level(LevelFilter::Trace);
    }

    let app = App {
        args: cli.clone(),
        client: None,
    };

    let res = match &cli.command {
        Commands::Completion { shell } => commands::completions::completions(*shell).await,
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
        Commands::Auth { command } => {
            // If not in verbose, set a higher default log level as the output matters
            if !cli.verbose {
                set_log_level(LevelFilter::Error);
            }
            match command {
                AuthCommands::Raw { client_id } => commands::auth::raw(app, client_id).await,
                AuthCommands::Kubectl { client_id } => {
                    commands::auth::kubectl(app, client_id).await
                }
                AuthCommands::Aws {
                    client_id,
                    role_arn,
                    region,
                } => commands::auth::aws(app, client_id, role_arn, region).await,
            }
        }
        Commands::Api { command } => api::exec_api_command(app, command).await,
    };
    match res {
        Ok(_) => Ok(()),
        Err(e) => {
            eprintln!("Error running command: {e:?}");
            Ok(())
        }
    }
}
