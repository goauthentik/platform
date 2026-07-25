use crate::agent::Agent;
use ak_platform::paths::sysd_config_file;
use clap::{Parser, Subcommand};
use eyre::Result;

pub mod agent;
pub mod cfg;
pub mod check;
pub mod cli;
pub mod components;
pub mod context;
pub mod events;
pub mod state;
pub mod util;

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
    /// Configure authentik domains
    #[command(subcommand)]
    Domains(DomainsCommands),
    /// Troubleshooting commands
    #[command(subcommand)]
    Troubleshoot(TroubleshootCommands),
    /// Version of authentik Agent components
    Version,
    /// Used as an OpenSSH AuthorizedPrincipalsCommand
    #[command(hide = true)]
    SshVerify {
        user: String,
        b64key: String,
        r#type: String,
    },
}

#[derive(Subcommand, Clone)]
enum DomainsCommands {
    /// Enroll this machine into an authentik domain
    Join {
        domain_name: String,
        /// URL to the authentik Instance
        #[arg(short = 'a', long = "authentik-url")]
        authentik_url: String,
    },
}

#[derive(Subcommand, Clone)]
enum TroubleshootCommands {
    /// Validate NSS/PAM setup and daemon reachability
    Check,
    /// Inspect facts
    Facts,
    /// Inspect state
    Inspect,
}

#[tokio::main]
pub async fn main() -> Result<()> {
    let cli = SysdArgs::parse();

    match cli.command {
        Commands::Agent => {
            let ag = Agent::new(cli.config).await?;
            ag.start().await?;
            ag.wait().await?;
        }
        Commands::Domains(DomainsCommands::Join {
            domain_name,
            authentik_url,
        }) => {
            self::cli::domains::join(domain_name, authentik_url).await?;
        }
        Commands::Troubleshoot(TroubleshootCommands::Check) => {
            self::cli::troubleshoot::check().await?;
        }
        Commands::Troubleshoot(TroubleshootCommands::Facts) => {
            self::cli::troubleshoot::facts().await?;
        }
        Commands::Troubleshoot(TroubleshootCommands::Inspect) => {
            self::cli::troubleshoot::inspect().await?;
        }
        Commands::Version => {
            self::cli::version::print_version().await?;
        }
        Commands::SshVerify {
            user,
            b64key,
            r#type,
        } => {
            self::cli::ssh_verify::verify(user, b64key, r#type).await;
        }
    }
    Ok(())
}
