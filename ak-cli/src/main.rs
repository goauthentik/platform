use crate::commands::{auth::AuthCommands, config::ConfigCommands};
use ak_platform::grpc::assert_response_valid;
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
    #[arg(short, long, default_value_t = false, global = true)]
    verbose: bool,
    /// Output JSON data
    #[arg(short, long, default_value_t = false, global = true)]
    json: bool,
    /// A name for the profile
    #[arg(short, long, global = true)]
    profile: Option<String>,
    /// Socket the agent is listening on
    #[arg(short, long, global = true)]
    socket: Option<String>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Clone)]
enum Commands {
    /// Check user account details for a given profile
    Whoami,
    /// Version of authentik Agent components
    Version,
    /// Switch to a different active profile
    #[command(alias = "s")]
    SwitchProfile {
        #[arg(required = true)]
        profile: String,
    },

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

#[derive(Clone)]
pub struct App {
    args: CliArgs,
    client: Option<Client<AnyService>>,
    active_profile: String,
}

impl App {
    pub async fn new(args: CliArgs) -> Self {
        let mut app = App {
            args,
            client: None,
            active_profile: "".to_string(),
        };
        let active_profile = app.lookup_profile().await;
        app.active_profile = active_profile;
        app
    }

    pub fn profile(&self) -> String {
        self.active_profile.clone()
    }

    async fn lookup_profile(&self) -> String {
        if let Some(p) = &self.args.profile {
            tracing::debug!(profile = p, "Using argument-specified profile");
            return p.clone();
        }
        let cp: Result<String> = async {
            let res = self
                .clone()
                .user()
                .await?
                .ctrl()
                .current_profile(())
                .await?
                .into_inner();
            assert_response_valid(res.header)?;
            Ok(res.profile)
        }
        .await;
        match cp {
            Ok(p) => {
                tracing::debug!(profile = p, "Using currently selected profile");
                p
            }
            Err(e) => {
                tracing::warn!("failed to get profile from agent: {e:?}");
                DEFAULT_PROFILE.to_string()
            }
        }
    }

    pub async fn user(self) -> Result<Client<AnyService>> {
        match self.client {
            Some(c) => Ok(c),
            None => Ok(Client::new(self.args.socket).await?),
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

    let app = App::new(cli.clone()).await;

    let res = match &cli.command {
        Commands::Completion { shell } => commands::completions::completions(*shell).await,
        Commands::Whoami => commands::whoami::whoami(app).await,
        Commands::Version => commands::version::version(app).await,
        Commands::SwitchProfile { profile } => commands::config::switch_profile(app, profile).await,
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
