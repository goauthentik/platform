use crate::format;
use crate::setup::ak::urls_for_profile;
use eyre::Result;
use oauth2::basic::BasicClient;
use oauth2::{
    ClientId, DeviceAuthorizationUrl, Scope,
    StandardDeviceAuthorizationResponse, TokenResponse, TokenUrl,
};
use url::Url;

use open::that;
use ratatui::text::Line;
use std::time::Duration;

pub mod ak;

type URLCallback = fn(url: Url) -> Result<()>;

pub struct Options {
    pub profile_name: String,
    pub authentik_url: Url,
    pub app_slug: String,
    pub client_id: String,
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

    let client = BasicClient::new(ClientId::new(opts.client_id.clone()))
        .set_token_uri(TokenUrl::from_url(urls.token_url))
        .set_device_authorization_url(DeviceAuthorizationUrl::from_url(urls.device_code_url));

    let reqwest_client = reqwest::ClientBuilder::new()
        // Following redirects opens the client up to SSRF vulnerabilities.
        .redirect(reqwest::redirect::Policy::none())
        .build()?;
    let http_client = ak_platform::oauth2_http::adapter(reqwest_client);

    let details: StandardDeviceAuthorizationResponse = client
        .exchange_device_code()
        .add_scopes(vec![
            Scope::new("openid".to_string()),
            Scope::new("profile".to_string()),
            Scope::new("email".to_string()),
            Scope::new("offline_access".to_string()),
            Scope::new("goauthentik.io/api".to_string()),
        ])
        .add_scope(Scope::new("read".to_string()))
        .request_async(&http_client)
        .await?;

    callback(details.verification_uri().url().clone())?;

    eprintln!("Waiting for authentication...");

    let token_response = client
        .exchange_device_access_token(&details)
        .request_async(
            &http_client,
            tokio::time::sleep,
            Some(Duration::from_secs(15)),
        )
        .await?;

    eprintln!("Successfully authenticated!");

    let mut profile = Profile {
        authentik_url: opts.authentik_url.clone(),
        app_slug: opts.app_slug.clone(),
        client_id: opts.client_id.clone(),
        access_token: Some(token_response.access_token().secret().clone()),
        refresh_token: None,
    };
    if let Some(token) = token_response.refresh_token() {
        profile.refresh_token = Some(token.secret().clone())
    }
    Ok(profile)
}
