use ak_cli::{
    Cli, Commands,
    commands::{auth::AuthCommands, config::ConfigCommands},
};
use akp_logger::set_log_level;
use clap::{Error, Parser};
use log::LevelFilter;

#[tokio::main]
async fn main() -> Result<(), Error> {
    let cli = Cli::parse();

    set_log_level(LevelFilter::Warn);
    if cli.verbose {
        set_log_level(LevelFilter::Trace);
    }

    let res = match &cli.command {
        Commands::Whoami => ak_cli::commands::whoami::whoami(&cli).await,
        Commands::Version => ak_cli::commands::version::version(&cli).await,
        Commands::Config { command } => match command {
            ConfigCommands::ListProfiles => ak_cli::commands::config::list_profiles(&cli).await,
        },
        Commands::Auth { command } => match command {
            AuthCommands::Raw { client_id } => {
                ak_cli::commands::auth::raw(&cli, client_id).await
            }
            AuthCommands::Kubectl { client_id } => {
                ak_cli::commands::auth::kubectl(&cli, client_id).await
            }
            AuthCommands::Aws {
                client_id,
                role_arn,
                region,
            } => ak_cli::commands::auth::aws(&cli, client_id, role_arn, region).await,
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
