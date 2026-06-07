use clap::Subcommand;
use std::error::Error;

use crate::{Cli, auth::aws, auth::raw};

#[derive(Subcommand)]
pub enum AuthCommands {
    /// Authenticate to arbitrary API calls.
    Raw {
        #[arg(short, long, required = true)]
        client_id: String,
    },
    Aws {
        #[arg(short, long, required = true)]
        client_id: String,
        #[arg(short, long, required = true)]
        role_arn: String,
        #[arg(short, long, required = true)]
        region: String,
    },
}

pub async fn raw(cli: &Cli, client_id: &str) -> Result<(), Box<dyn Error>> {
    let creds = raw::get_credentials(raw::CredentialsOpts {
        profile: cli.profile.clone(),
        client_id: client_id.to_owned(),
    })
    .await?;
    println!("{}", creds.access_token);
    Ok(())
}

pub async fn aws(
    cli: &Cli,
    client_id: &str,
    role_arn: &str,
    region: &str,
) -> Result<(), Box<dyn Error>> {
    let creds = aws::get_credentials(aws::CredentialsOpts {
        profile: cli.profile.clone(),
        client_id: client_id.to_owned(),
        role_arn: role_arn.to_owned(),
        region: region.to_owned(),
    })
    .await?;
    print!("{}", serde_json::to_string(&creds)?);
    Ok(())
}
