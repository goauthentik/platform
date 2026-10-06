use crate::setup::ak::urls_for_profile;
use eyre::{Result, WrapErr};
use oauth2::basic::BasicClient;
use oauth2::{
    ClientId, DeviceAuthorizationUrl, Scope, StandardDeviceAuthorizationResponse, TokenResponse,
    TokenUrl,
};
use url::Url;

pub mod ak;

pub struct Options {
    pub authentik_url: Url,
    pub app_slug: String,
    pub client_id: String,
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

/// Run the OAuth device flow. `url_callback` receives the verification URL
/// the user must open to authorize the device.
pub async fn setup(opts: Options, url_callback: impl FnOnce(Url) -> Result<()>) -> Result<Profile> {
    let urls = urls_for_profile(Profile::new(
        opts.authentik_url.clone(),
        opts.app_slug.clone(),
        opts.client_id.clone(),
    ))?;

    let client = BasicClient::new(ClientId::new(opts.client_id.clone()))
        .set_token_uri(TokenUrl::from_url(urls.token_url))
        .set_device_authorization_url(DeviceAuthorizationUrl::from_url(urls.device_code_url));

    let reqwest_client = reqwest::ClientBuilder::new()
        // Following redirects opens the client up to SSRF vulnerabilities.
        .redirect(reqwest::redirect::Policy::none())
        .build()?;
    let http_client = crate::oauth2_http::adapter(reqwest_client);

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

    let verification_url = match details.verification_uri_complete() {
        Some(vu) => Url::parse(vu.secret()).wrap_err("invalid verification URI")?,
        None => details.verification_uri().url().clone(),
    };
    url_callback(verification_url)?;

    let token_response = client
        .exchange_device_access_token(&details)
        .request_async(&http_client, tokio::time::sleep, None)
        .await?;

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
