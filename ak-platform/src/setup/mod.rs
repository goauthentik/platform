use crate::dpop::DpopKeyPair;
use crate::oauth::device_flow::{OAuthError, poll_for_device_token, request_device_authorization};
use crate::setup::ak::urls_for_profile;
use eyre::Result;
use url::Url;

pub mod ak;

const SCOPE_BOUND_KEY: &str = "bound_key";

pub struct Options {
    pub authentik_url: Url,
    pub app_slug: String,
    pub client_id: String,
    pub user_agent: String,
}

pub struct Profile {
    pub authentik_url: Url,
    pub app_slug: String,
    pub client_id: String,
    pub access_token: Option<String>,
    pub refresh_token: Option<String>,
    /// PKCS#8 PEM DPoP private key, when the server bound the tokens to it.
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

    // Always attempt key binding and let the server decide: authentik
    // >= 2026.8 rejects `dpop_jkt` when the provider has no `bound_key` scope
    // mapping, and older versions silently drop the scope. Either way the
    // granted scopes in the token response tell us whether binding happened.
    let dpop_keypair = DpopKeyPair::generate();
    let dpop_jkt = dpop_keypair.thumbprint()?;
    let mut scopes = vec![
        "openid",
        "profile",
        "email",
        "offline_access",
        "goauthentik.io/api",
        SCOPE_BOUND_KEY,
    ];

    let mut dpop_requested = true;
    let auth = match request_device_authorization(
        &urls.device_code_url,
        &opts.client_id,
        &scopes,
        Some(&dpop_jkt),
        &opts.user_agent,
    )
    .await
    {
        Err(e)
            if e.downcast_ref::<OAuthError>()
                .is_some_and(|e| e.0 == "dpop_jkt_not_allowed") =>
        {
            tracing::debug!("provider does not support key binding, continuing without DPoP");
            dpop_requested = false;
            scopes.retain(|s| *s != SCOPE_BOUND_KEY);
            request_device_authorization(
                &urls.device_code_url,
                &opts.client_id,
                &scopes,
                None,
                &opts.user_agent,
            )
            .await?
        }
        res => res?,
    };

    url_callback(
        auth.verification_uri_complete
            .clone()
            .unwrap_or_else(|| auth.verification_uri.clone()),
    )?;

    let token_response = poll_for_device_token(
        &urls.token_url,
        &opts.client_id,
        &auth,
        dpop_requested.then_some(&dpop_keypair),
        &opts.user_agent,
    )
    .await?;

    let dpop_bound = token_response
        .scope
        .as_deref()
        .is_some_and(|s| s.split_whitespace().any(|s| s == SCOPE_BOUND_KEY));
    let dpop_private_key_pem = if dpop_bound {
        Some(dpop_keypair.to_pkcs8_pem()?)
    } else {
        None
    };

    Ok(Profile {
        authentik_url: opts.authentik_url.clone(),
        app_slug: opts.app_slug.clone(),
        client_id: opts.client_id.clone(),
        access_token: Some(token_response.access_token),
        refresh_token: token_response.refresh_token,
        dpop_private_key_pem,
    })
}
