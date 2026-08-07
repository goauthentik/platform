use crate::format;
use crate::setup::ak::urls_for_profile;
use ak_meta::user_agent;
use ak_platform::client::user::{AnyService, Client};
use ak_platform::dpop::DpopProver;
use ak_platform::generated::agent::RequestHeader;
use ak_platform::generated::agent_auth::SignDpopProofRequest;
use ak_platform::generated::agent_auth::agent_auth_client::AgentAuthClient;
use ak_platform::generated::agent_ctrl::PrepareDpopKeyRequest;
use ak_platform::oauth::device_flow::{poll_for_device_token, request_device_authorization};
use eyre::{Result, WrapErr};
use open::that;
use ratatui::text::Line;
use url::Url;

pub mod ak;

type URLCallback = fn(url: Url) -> Result<()>;

pub struct Options {
    pub profile_name: String,
    pub authentik_url: Url,
    pub app_slug: String,
    pub client_id: String,
    pub dpop_enabled: bool,
    pub agent: Client<AnyService>,
    pub url_callback: Option<URLCallback>,
}

pub struct Profile {
    pub authentik_url: Url,
    pub app_slug: String,
    pub client_id: String,
    pub access_token: Option<String>,
    pub refresh_token: Option<String>,
}

impl Profile {
    pub fn new(authentik_url: Url, app_slug: String, client_id: String) -> Profile {
        Profile {
            authentik_url,
            app_slug,
            client_id,
            access_token: None,
            refresh_token: None,
        }
    }
}

/// Signs DPoP proofs by asking ak-agent over gRPC — the device flow here runs
/// from ak-cli, which never holds the profile's DPoP key itself (it may be
/// hardware-backed and non-exportable; ak-agent is the sole owner).
struct RpcDpopProver {
    auth: AgentAuthClient<AnyService>,
    profile_name: String,
}

#[tonic::async_trait]
impl DpopProver for RpcDpopProver {
    async fn prove(&self, htm: &str, htu: &str, code_for_c_s256: Option<&str>) -> Result<String> {
        let mut auth = self.auth.clone();
        let res = auth
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

pub async fn setup(opts: Options) -> Result<Profile> {
    let urls = urls_for_profile(Profile::new(
        opts.authentik_url.clone(),
        opts.app_slug.clone(),
        opts.client_id.clone(),
    ))?;
    let callback: URLCallback = match opts.url_callback {
        Some(c) => c,
        None => |url: Url| -> Result<()> {
            match that(url.to_string()) {
                Ok(_) => Ok(()),
                Err(e) => {
                    tracing::debug!("failed to open URL in browser: {e:?}");
                    println!(
                        "{}",
                        Line::styled(
                            format!("Open this URL in your browser: {}", url),
                            format::box_style()
                        )
                    );
                    Ok(())
                }
            }
        },
    };

    let mut dpop_jkt = None;
    let mut dpop_prover: Option<RpcDpopProver> = None;
    if opts.dpop_enabled {
        let res = opts
            .agent
            .clone()
            .ctrl()
            .prepare_dpop_key(PrepareDpopKeyRequest {
                header: Some(RequestHeader {
                    profile: opts.profile_name.clone(),
                }),
                authentik_url: opts.authentik_url.to_string(),
                app_slug: opts.app_slug.clone(),
                client_id: opts.client_id.clone(),
            })
            .await
            .wrap_err("failed to prepare DPoP key")?
            .into_inner();

        if res.hardware_backed {
            eprintln!("Enrolling with a hardware-backed DPoP key.");
        } else {
            eprintln!(
                "Hardware key storage unavailable on this device — using a software-protected DPoP key."
            );
        }

        dpop_jkt = Some(res.dpop_jkt);
        dpop_prover = Some(RpcDpopProver {
            auth: opts.agent.clone().auth(),
            profile_name: opts.profile_name.clone(),
        });
    }

    let mut scopes = vec![
        "openid",
        "profile",
        "email",
        "offline_access",
        "goauthentik.io/api",
    ];
    if opts.dpop_enabled {
        scopes.push("bound_key");
    }

    let auth = request_device_authorization(
        &urls.device_code_url,
        &opts.client_id,
        &scopes,
        dpop_jkt.as_deref(),
        &user_agent(),
    )
    .await?;

    let verification_uri = auth
        .verification_uri_complete
        .clone()
        .unwrap_or_else(|| auth.verification_uri.clone());
    callback(verification_uri)?;

    eprintln!("Waiting for authentication...");
    let token_response = poll_for_device_token(
        &urls.token_url,
        &opts.client_id,
        &auth,
        dpop_prover.as_ref().map(|p| p as &dyn DpopProver),
        &user_agent(),
    )
    .await?;

    Ok(Profile {
        authentik_url: opts.authentik_url.clone(),
        app_slug: opts.app_slug.clone(),
        client_id: opts.client_id.clone(),
        access_token: Some(token_response.access_token),
        refresh_token: token_response.refresh_token,
    })
}
