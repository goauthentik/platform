use crate::format;
use crate::setup::ak::urls_for_profile;
use ak_meta::user_agent;
use ak_platform::dpop::DpopKeyPair;
use ak_platform::oauth::device_flow::{poll_for_device_token, request_device_authorization};
use eyre::Result;
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
    pub url_callback: Option<URLCallback>,
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
        dpop_keypair.as_ref(),
        &user_agent(),
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
