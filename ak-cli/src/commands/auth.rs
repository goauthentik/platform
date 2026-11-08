use crate::{
    App,
    auth::{aws, claude, k8s, raw, vault},
};
use clap::Subcommand;
use eyre::{Result, WrapErr};

#[derive(Subcommand, Clone)]
pub enum AuthCommands {
    /// Authenticate to arbitrary API calls.
    Raw {
        /// Client ID to exchange a token for
        #[arg(short, long, required = true)]
        client_id: String,
    },
    /// Authenticate to AWS with the authentik profile.
    Aws {
        #[clap(flatten)]
        provider: aws::AWSProviderArgs,
    },
    /// Authenticate to an AWS EKS cluster, replaces the need for `aws eks get-token`
    AwsEks {
        #[clap(flatten)]
        provider: aws::AWSProviderArgs,
        /// Name of the EKS cluster
        #[arg(short = 'l', long, required = true)]
        cluster: String,
    },
    /// Authenticate to a Kubernetes Cluster with the authentik profile.
    Kubectl {
        /// Client ID of the Kubernetes authentik provider
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
    /// Run the Vault/OpenBao CLI authenticated via a JWT auth role, meant to be aliased:
    /// `alias vault='ak auth vault -c <id> -r <role> -a <addr> --'`.
    Vault {
        #[arg(short, long, required = true)]
        client_id: String,
        #[arg(short, long, required = true)]
        role: String,
        #[arg(short, long, default_value = "jwt")]
        mount: String,
        #[arg(short, long, env = "VAULT_ADDR")]
        address: String,
        /// CLI to run, e.g. `bao` for OpenBao
        #[arg(short, long, default_value = "vault")]
        bin: String,
        /// Arguments passed to the CLI, after `--`
        #[arg(last = true)]
        args: Vec<String>,
    },
}

pub async fn handle(cmd: AuthCommands, app: App) -> Result<()> {
    match cmd {
        AuthCommands::Raw { client_id } => raw(app, client_id).await,
        AuthCommands::Kubectl { client_id } => kubectl(app, client_id).await,
        AuthCommands::Aws { provider } => aws(app, provider).await,
        AuthCommands::AwsEks { provider, cluster } => aws_eks(app, provider, cluster).await,
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
        AuthCommands::Vault {
            client_id,
            role,
            mount,
            address,
            bin,
            args,
        } => vault(app, client_id, address, mount, role, bin, args).await,
    }
}

async fn vault(
    mut app: App,
    client_id: String,
    address: String,
    mount: String,
    role: String,
    bin: String,
    args: Vec<String>,
) -> Result<()> {
    let creds = vault::get_credentials(
        app.clone().user().await?,
        vault::CredentialsOpts {
            profile: app.profile().await,
            client_id,
            address: address.clone(),
            mount,
            role,
        },
    )
    .await?;
    let status = std::process::Command::new(&bin)
        .args(args)
        .env("VAULT_ADDR", &address)
        .env("VAULT_TOKEN", &creds.client_token)
        .env("BAO_ADDR", &address)
        .env("BAO_TOKEN", &creds.client_token)
        .status()
        .wrap_err_with(|| format!("failed to run {bin}"))?;
    std::process::exit(status.code().unwrap_or(1));
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

async fn aws(mut app: App, provider: aws::AWSProviderArgs) -> Result<()> {
    let creds = aws::get_credentials(
        app.clone().user().await?,
        aws::CredentialsOpts {
            profile: app.profile().await,
            provider_lookup: provider,
        },
    )
    .await?;
    print!("{}", serde_json::to_string(&creds)?);
    Ok(())
}

async fn aws_eks(mut app: App, provider: aws::AWSProviderArgs, cluster: String) -> Result<()> {
    let creds = aws::get_eks_credentials(
        app.clone().user().await?,
        aws::EKSCredentialOpts {
            opts: aws::CredentialsOpts {
                profile: app.profile().await,
                provider_lookup: provider,
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
