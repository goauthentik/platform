use crate::dpop::DpopKeyPair;
use crate::oauth::device_flow::{poll_for_device_token, request_device_authorization};
use crate::setup::ak::urls_for_profile;
use eyre::Result;
use url::Url;

pub mod ak;

pub struct Options {
    pub authentik_url: Url,
    pub app_slug: String,
    pub client_id: String,
    pub dpop_enabled: bool,
    pub user_agent: String,
}

pub struct Profile {
    pub authentik_url: Url,
    pub app_slug: String,
    pub client_id: String,
    pub access_token: Option<String>,
    pub refresh_token: Option<String>,
    /// PKCS#8 PEM DPoP private key, when `Options::dpop_enabled` was set.
    pub dpop_private_key_pem: Option<String>,
}

impl Profile {
    pub fn new(authentik_url: Url, app_slug: String, client_id: String) -> Profile {
        Profile {
            authentik_url,
            app_slug,
            client_id,
            access_token: None,
            refresh_token: None,
            dpop_private_key_pem: None,
        }
    }
}

/// Run the OAuth device flow. `url_callback` receives the verification URL
/// the user must open to authorize the device.
pub async fn setup(opts: Options, url_callback: impl FnOnce(Url) -> Result<()>) -> Result<Profile> {
    let urls = urls_for_profile(Profile::new(
        opts.authentik_url.clone(),
        opts.app_slug.clone(),
        opts.client_id.clone(),
    ))?;

    let dpop_keypair = opts.dpop_enabled.then(DpopKeyPair::generate);
    let dpop_jkt = dpop_keypair
        .as_ref()
        .map(DpopKeyPair::thumbprint)
        .transpose()?;

    let mut scopes = vec![
        "openid",
        "profile",
        "email",
        "offline_access",
        "goauthentik.io/api",
        "read",
    ];
    if opts.dpop_enabled {
        scopes.push("bound_key");
    }

    let auth = request_device_authorization(
        &urls.device_code_url,
        &opts.client_id,
        &scopes,
        dpop_jkt.as_deref(),
        &opts.user_agent,
    )
    .await?;

    url_callback(
        auth.verification_uri_complete
            .clone()
            .unwrap_or_else(|| auth.verification_uri.clone()),
    )?;

    let token_response = poll_for_device_token(
        &urls.token_url,
        &opts.client_id,
        &auth,
        dpop_keypair.as_ref(),
        &opts.user_agent,
    )
    .await?;

    let dpop_private_key_pem = dpop_keypair
        .as_ref()
        .map(DpopKeyPair::to_pkcs8_pem)
        .transpose()?;

    Ok(Profile {
        authentik_url: opts.authentik_url.clone(),
        app_slug: opts.app_slug.clone(),
        client_id: opts.client_id.clone(),
        access_token: Some(token_response.access_token),
        refresh_token: token_response.refresh_token,
        dpop_private_key_pem,
    })
}
