use crate::{
    App,
    auth::{aws, k8s, raw},
};
use ak_platform::prelude::*;
use clap::Subcommand;

#[derive(Subcommand, Clone)]
pub enum AuthCommands {
    /// Authenticate to arbitrary API calls.
    Raw {
        #[arg(short, long, required = true)]
        client_id: String,
    },
    /// Authenticate to AWS with the authentik profile.
    Aws {
        #[arg(short, long, required = true)]
        client_id: String,
        #[arg(short, long, required = true)]
        role_arn: String,
        #[arg(short, long, required = true)]
        region: String,
    },
    /// Authenticate to a Kubernetes Cluster with the authentik profile.
    Kubectl {
        #[arg(short, long, required = true)]
        client_id: String,
    },
}

pub async fn raw(app: App, client_id: &str) -> Result<()> {
    let creds = raw::get_credentials(
        app.clone().user().await?,
        raw::CredentialsOpts {
            profile: app.args.profile.clone(),
            client_id: client_id.to_owned(),
        },
    )
    .await?;
    println!("{}", creds.access_token);
    Ok(())
}

pub async fn aws(app: App, client_id: &str, role_arn: &str, region: &str) -> Result<()> {
    let creds = aws::get_credentials(
        app.clone().user().await?,
        aws::CredentialsOpts {
            profile: app.args.profile.clone(),
            client_id: client_id.to_owned(),
            role_arn: role_arn.to_owned(),
            region: region.to_owned(),
        },
    )
    .await?;
    print!("{}", serde_json::to_string(&creds)?);
    Ok(())
}

pub async fn kubectl(app: App, client_id: &str) -> Result<()> {
    let creds = k8s::get_credentials(
        app.clone().user().await?,
        k8s::CredentialsOpts {
            profile: app.args.profile.clone(),
            client_id: client_id.to_owned(),
        },
    )
    .await?;
    print!("{}", serde_json::to_string(&creds)?);
    Ok(())
}
