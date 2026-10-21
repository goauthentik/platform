use crate::{
    App,
    auth::{aws, claude, k8s, raw},
};
use clap::Subcommand;
use eyre::Result;

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
        #[arg(short = 'e', long, required = true)]
        region: String,
    },
    /// Authenticate to a Kubernetes Cluster with the authentik profile.
    Kubectl {
        #[arg(short, long, required = true)]
        client_id: String,
    },
    /// Claude
    Claude {
        #[arg(short, long, required = true)]
        client_id: String,
        #[arg(short, long, required = true)]
        federation_rule_id: String,
        #[arg(short, long, required = true)]
        organization_id: String,
        #[arg(short = 'a', long, required = true)]
        service_account_id: String,
        #[arg(short, long, required = true)]
        workspace_id: String,
    },
}

pub async fn raw(mut app: App, client_id: &str) -> Result<()> {
    let creds = raw::get_credentials(
        app.clone().user().await?,
        raw::CredentialsOpts {
            profile: app.profile().await,
            client_id: client_id.to_owned(),
        },
    )
    .await?;
    println!("{}", creds.access_token);
    Ok(())
}

pub async fn aws(mut app: App, client_id: &str, role_arn: &str, region: &str) -> Result<()> {
    let creds = aws::get_credentials(
        app.clone().user().await?,
        aws::CredentialsOpts {
            profile: app.profile().await,
            client_id: client_id.to_owned(),
            role_arn: role_arn.to_owned(),
            region: region.to_owned(),
        },
    )
    .await?;
    print!("{}", serde_json::to_string(&creds)?);
    Ok(())
}

pub async fn kubectl(mut app: App, client_id: &str) -> Result<()> {
    let creds = k8s::get_credentials(
        app.clone().user().await?,
        k8s::CredentialsOpts {
            profile: app.profile().await,
            client_id: client_id.to_owned(),
        },
    )
    .await?;
    print!("{}", serde_json::to_string(&creds)?);
    Ok(())
}

pub async fn claude(
    mut app: App,
    client_id: &str,
    federation_rule_id: &str,
    organization_id: &str,
    service_account_id: &str,
    workspace_id: &str,
) -> Result<()> {
    let creds = claude::get_credentials(
        app.clone().user().await?,
        claude::CredentialsOpts {
            profile: app.profile().await,
            client_id: client_id.to_owned(),
            federation_rule_id: federation_rule_id.to_owned(),
            organization_id: organization_id.to_owned(),
            service_account_id: service_account_id.to_owned(),
            workspace_id: workspace_id.to_owned(),
        },
    )
    .await?;
    print!("{}", creds.access_token);
    Ok(())
}
