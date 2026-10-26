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
        #[clap(flatten)]
        provider: aws::AWSProviderArgs,
        #[arg(short, long, required = true)]
        role_arn: String,
        #[arg(short = 'e', long, required = true)]
        region: String,
    },
    /// Authenticate to an AWS EKS cluster, requires the need for `aws eks get-token`
    AwsEks {
        #[clap(flatten)]
        provider: aws::AWSProviderArgs,
        #[arg(short, long, required = true)]
        role_arn: String,
        #[arg(short = 'e', long, required = true)]
        region: String,
        #[arg(short = 'l', long, required = true)]
        cluster: String,
    },
    /// Authenticate to a Kubernetes Cluster with the authentik profile.
    Kubectl {
        #[arg(short, long, required = true)]
        client_id: String,
    },
    /// Authenticate to Anthropic's Claude using Workload Identity Federation.
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

pub async fn handle(cmd: AuthCommands, app: App) -> Result<()> {
    match cmd {
        AuthCommands::Raw { client_id } => raw(app, client_id).await,
        AuthCommands::Kubectl { client_id } => kubectl(app, client_id).await,
        AuthCommands::Aws {
            provider,
            role_arn,
            region,
        } => aws(app, provider, role_arn, region).await,
        AuthCommands::AwsEks {
            provider,
            role_arn,
            region,
            cluster,
        } => aws_eks(app, provider, role_arn, region, cluster).await,
        AuthCommands::Claude {
            client_id,
            federation_rule_id,
            organization_id,
            service_account_id,
            workspace_id,
        } => {
            claude(
                app,
                client_id,
                federation_rule_id,
                organization_id,
                service_account_id,
                workspace_id,
            )
            .await
        }
    }
}

async fn raw(mut app: App, client_id: String) -> Result<()> {
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

async fn aws(
    mut app: App,
    provider: aws::AWSProviderArgs,
    role_arn: String,
    region: String,
) -> Result<()> {
    let creds = aws::get_credentials(
        app.clone().user().await?,
        aws::CredentialsOpts {
            profile: app.profile().await,
            provider_lookup: provider,
            role_arn: role_arn.to_owned(),
            region: region.to_owned(),
        },
    )
    .await?;
    print!("{}", serde_json::to_string(&creds)?);
    Ok(())
}

async fn aws_eks(
    mut app: App,
    provider: aws::AWSProviderArgs,
    role_arn: String,
    region: String,
    cluster: String,
) -> Result<()> {
    let creds = aws::get_eks_credentials(
        app.clone().user().await?,
        aws::EKSCredentialOpts {
            opts: aws::CredentialsOpts {
                profile: app.profile().await,
                provider_lookup: provider,
                role_arn: role_arn.to_owned(),
                region: region.to_owned(),
            },
            cluster,
        },
    )
    .await?;
    print!("{}", serde_json::to_string(&creds)?);
    Ok(())
}

async fn kubectl(mut app: App, client_id: String) -> Result<()> {
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

async fn claude(
    mut app: App,
    client_id: String,
    federation_rule_id: String,
    organization_id: String,
    service_account_id: String,
    workspace_id: String,
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
