use std::error::Error;
use clap::Subcommand;

use crate::{Cli, auth::raw::{CredentialsOpts, get_credentials}};


#[derive(Subcommand)]
pub enum AuthCommands {
    /// Authenticate to arbitrary API calls.
    Raw {
        #[arg(short, long, required = true)]
        client_id: String,
    },
}

pub async fn raw(cli: &Cli, client_id: &String) -> Result<(), Box<dyn Error>> {
    let creds = get_credentials(CredentialsOpts {
        client_id: client_id.clone(),
        profile: cli.profile.clone(),
    }).await?;
    println!("{}", creds.access_token);
    Ok(())
}
