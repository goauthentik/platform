use ak_meta::full_version;
use ak_platform::{
    log::{LevelFilter, LogBuilder},
    paths::sysd_config_file,
    string::PlatformString,
};
use clap::{Parser, Subcommand};
use clap_complete::Shell;
use eyre::Result;

/// See the `tikv-jemallocator` dependency comment in `Cargo.toml`: glibc
/// strands the in-process osquery engine's large transient allocations in its
/// arenas forever, where jemalloc decays them back to the OS.
#[cfg(not(target_os = "windows"))]
#[global_allocator]
static GLOBAL: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;

pub mod agent;
pub mod cfg;
pub mod check;
pub mod cli;
pub mod components;
pub mod context;
pub mod events;
pub mod runner;
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
    /// Enable debug logging
    #[arg(short, default_value_t = false)]
    debug: bool,
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
    /// Generate shell completion scripts
    Completion {
        /// Shell to generate completions for
        shell: Shell,
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

#[ak_meta::main("ak-sysd")]
pub async fn main() -> Result<()> {
    let cli = SysdArgs::parse();
    LogBuilder::new(
        PlatformString::new()
            .with_linux("ak-sysd")
            .with_windows("authentik System Service"),
    )
    .allow_platform(true)
    .allow_stdout(true)
    .default_level(LevelFilter::Info)
    .with_filter(
        "ak_sysd",
        match cli.debug {
            true => LevelFilter::Trace,
            false => LevelFilter::Info,
        },
    )
    .enable();
    tracing::trace!("authentik sysd v{}", full_version());

    match cli.command {
        Commands::Agent => {
            self::runner::run(cli.config).await?;
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
        Commands::Completion { shell } => self::cli::completions::completions(shell).await?,
    }
    Ok(())
}
