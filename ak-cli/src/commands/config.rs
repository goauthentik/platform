use crate::{
    App,
    format::{self, render_timestamp},
};
use ak_platform::{
    client::user::AnyService,
    dpop::DpopProver,
    generated::{
        agent::RequestHeader,
        agent_auth::{SignDpopProofRequest, agent_auth_client::AgentAuthClient},
        agent_ctrl::{PrepareDpopKeyRequest, SetupRequest},
    },
    grpc::assert_response_valid,
    setup::{
        self,
        ak::{DEFAULT_APP_SLUG, DEFAULT_CLIENT_ID},
    },
};
use clap::Subcommand;
use eyre::{Result, WrapErr, bail};
use ratatui::text::Line;
use std::env;
use url::Url;

#[derive(Subcommand, Clone)]
pub enum ConfigCommands {
    /// List profiles
    ListProfiles,
    /// Delete a profile
    DeleteProfile {
        #[arg(required = true)]
        profile: String,
    },
    /// Configure authentik CLI
    Setup {
        #[arg(short, long, required = true)]
        authentik_url: String,
        #[arg(short = 'i', long, default_value = DEFAULT_CLIENT_ID)]
        client_id: String,
        #[arg(short = 'd', long, default_value = DEFAULT_APP_SLUG)]
        app_slug: String,
    },
}

pub async fn list_profiles(app: App) -> Result<()> {
    let res = app
        .user()
        .await?
        .clone()
        .ctrl()
        .list_profiles(())
        .await
        .wrap_err("failed to list profiles")?
        .into_inner();
    assert_response_valid(res.header)?;
    for profile in res.profiles {
        println!(
            "{}:",
            Line::styled(profile.name.to_string(), format::inline_style())
        );
        println!("\tUsername: {}", profile.username);
        println!("\tLast Renewal: {}", render_timestamp(profile.last_renewed));
        println!("\tNext Renewal: {}", render_timestamp(profile.next_renew));
        println!("\tauthentik URL: {}", profile.authentik_url);
        println!("\tDPoP bound: {}", profile.dpop_bound);
    }
    Ok(())
}

/// Signs DPoP proofs by asking ak-agent over gRPC: ak-cli never holds the
/// profile's DPoP key itself (it may be hardware-backed and non-exportable).
struct RpcDpopProver {
    auth: AgentAuthClient<AnyService>,
    profile_name: String,
}

#[tonic::async_trait]
impl DpopProver for RpcDpopProver {
    async fn prove(&self, htm: &str, htu: &str, code_for_c_s256: Option<&str>) -> Result<String> {
        let res = self
            .auth
            .clone()
            .sign_dpop_proof(SignDpopProofRequest {
                header: Some(RequestHeader {
                    profile: self.profile_name.clone(),
                }),
                htm: htm.to_string(),
                htu: htu.to_string(),
                code_for_c_s256: code_for_c_s256.map(|s| s.to_string()),
            })
            .await
            .wrap_err("failed to sign DPoP proof")?
            .into_inner();
        Ok(res.proof)
    }
}

pub async fn setup(
    mut app: App,
    authentik_url: &str,
    client_id: &str,
    app_slug: &str,
) -> Result<()> {
    let access_token: String;
    let refresh_token: String;
    let mut dpop_bound = false;
    if let Ok(at) = env::var("AK_CLI_ACCESS_TOKEN")
        && let Ok(rt) = env::var("AK_CLI_REFRESH_TOKEN")
    {
        access_token = at;
        refresh_token = rt;
    } else {
        let profile_name = app.args.profile.clone().unwrap_or(app.profile().await);
        let agent = app.clone().user().await?;
        let key = agent
            .clone()
            .ctrl()
            .prepare_dpop_key(PrepareDpopKeyRequest {
                header: Some(RequestHeader {
                    profile: profile_name.clone(),
                }),
                authentik_url: authentik_url.to_owned(),
                app_slug: app_slug.to_owned(),
                client_id: client_id.to_owned(),
            })
            .await
            .wrap_err("failed to prepare DPoP key")?
            .into_inner();
        assert_response_valid(key.header)?;
        tracing::debug!(hardware_backed = key.hardware_backed, "prepared DPoP key");
        let prover = RpcDpopProver {
            auth: agent.auth(),
            profile_name,
        };
        let prof = setup::setup(
            setup::Options {
                authentik_url: Url::parse(authentik_url).wrap_err("invalid authentik URL")?,
                app_slug: app_slug.to_owned(),
                client_id: client_id.to_owned(),
                user_agent: ak_meta::user_agent(),
            },
            setup::DpopKey {
                jkt: key.dpop_jkt,
                prover: &prover,
            },
            |url| {
                if let Err(e) = open::that(url.to_string()) {
                    tracing::debug!("failed to open URL in browser: {e:?}");
                    println!(
                        "{}",
                        Line::styled(
                            format!("Open this URL in your browser: {}", url),
                            format::box_style()
                        )
                    );
                }
                eprintln!("Waiting for authentication...");
                Ok(())
            },
        )
        .await
        .wrap_err("device flow setup failed")?;
        eprintln!("Successfully authenticated!");
        if let Some(at) = prof.access_token
            && let Some(rt) = prof.refresh_token
        {
            access_token = at;
            refresh_token = rt;
        } else {
            bail!("Device-flow setup did not return access/refresh token");
        }
        dpop_bound = prof.dpop_bound;
    }

    let res = app
        .clone()
        .user()
        .await?
        .ctrl()
        .setup(SetupRequest {
            header: Some(RequestHeader {
                profile: app.args.profile.clone().unwrap_or(app.profile().await),
            }),
            authentik_url: authentik_url.to_owned(),
            app_slug: app_slug.to_owned(),
            client_id: client_id.to_owned(),
            access_token: access_token.clone(),
            refresh_token: refresh_token.clone(),
            dpop_bound,
        })
        .await
        .wrap_err("failed to register profile with agent")?
        .into_inner();
    assert_response_valid(res.header)?;

    Ok(())
}

pub async fn switch_profile(app: App, profile: &Option<String>) -> Result<()> {
    let mut ctrl = app.user().await?.clone().ctrl();
    match profile {
        Some(p) => {
            let res = ctrl
                .switch_profile(RequestHeader {
                    profile: p.to_string(),
                })
                .await
                .wrap_err("failed to switch profile")?
                .into_inner();
            assert_response_valid(Some(res))?;
            println!("Successfully switched to profile '{p}'!");
            Ok(())
        }
        None => {
            let res = ctrl
                .current_profile(())
                .await
                .wrap_err("failed to get current profile")?
                .into_inner();
            assert_response_valid(res.header)?;
            println!("{}", res.profile);
            Ok(())
        }
    }
}

pub async fn delete_profile(app: App, profile: &str) -> Result<()> {
    let res = app
        .user()
        .await?
        .clone()
        .ctrl()
        .delete_profile(RequestHeader {
            profile: profile.to_string(),
        })
        .await
        .wrap_err("failed to delete profile")?
        .into_inner();
    assert_response_valid(Some(res))?;
    println!("Successfully deleted profile '{profile}'!");
    Ok(())
}
