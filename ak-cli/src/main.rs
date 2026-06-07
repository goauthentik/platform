use akp_logger::set_log_level;
use clap::{CommandFactory, Error, Parser, Subcommand};
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

    #[command(hide = true)]
    GenerateCompletions {
        shell: clap_complete::Shell,
    },
    #[command(hide = true)]
    GenerateManpage {
        out_dir: std::path::PathBuf,
    },
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    let cli = Cli::parse();

    set_log_level(LevelFilter::Warn);
    if cli.verbose {
        set_log_level(LevelFilter::Trace);
    }

    if let Commands::GenerateCompletions { shell } = &cli.command {
        clap_complete::generate(*shell, &mut Cli::command(), "ak", &mut std::io::stdout());
        return Ok(());
    }
    if let Commands::GenerateManpage { out_dir } = &cli.command {
        let man = clap_mangen::Man::new(Cli::command());
        let mut buf: Vec<u8> = Default::default();
        man.render(&mut buf).expect("man page generation failed");
        std::fs::write(out_dir.join("ak.1"), buf).expect("failed to write man page");
        return Ok(());
    }

    let res = match &cli.command {
        Commands::Whoami => commands::whoami::whoami(&cli).await,
        Commands::Version => commands::version::version(&cli).await,
        Commands::Config { command } => match command {
            ConfigCommands::ListProfiles => commands::config::list_profiles(&cli).await,
        },
        Commands::Auth { command } => match command {
            AuthCommands::Raw { client_id } => commands::auth::raw(&cli, client_id).await,
            AuthCommands::Kubectl { client_id } => commands::auth::kubectl(&cli, client_id).await,
            AuthCommands::Aws {
                client_id,
                role_arn,
                region,
            } => commands::auth::aws(&cli, client_id, role_arn, region).await,
        },
        Commands::GenerateCompletions { .. } | Commands::GenerateManpage { .. } => unreachable!(),
    };
    match res {
        Ok(_) => Ok(()),
        Err(e) => {
            eprintln!("Error running command: {e:?}");
            Ok(())
        }
    }
}
