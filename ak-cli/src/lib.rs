pub mod auth;
pub mod cache;
pub mod commands;
pub mod format;

use clap::{Parser, Subcommand};

use crate::commands::{auth::AuthCommands, config::ConfigCommands};

#[derive(Parser)]
#[command(name = "ak")]
#[command(version, about, long_about = None)]
#[command(propagate_version = true)]
pub struct Cli {
    /// Enable debug logging
    #[arg(short, long, default_value_t = false)]
    pub verbose: bool,
    /// Output JSON data
    #[arg(short, long, default_value_t = false)]
    pub json: bool,
    /// A name for the profile
    #[arg(short, long, default_value = "default")]
    pub profile: String,
    /// Socket the agent is listening on
    #[arg(short, long)]
    pub socket: Option<String>,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
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
