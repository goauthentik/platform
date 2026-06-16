use crate::format;
use crate::setup::ak::urls_for_profile;
use ak_platform::prelude::*;
use oauth_device_flows::provider::GenericProviderConfig;
use oauth_device_flows::{DeviceFlow, DeviceFlowConfig, Provider};
use open::that;
use ratatui::text::Line;
use std::time::Duration;
use url::Url;

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
                    log::debug!("failed to open URL in browser: {e:?}");
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

    let config = DeviceFlowConfig::new()
        .client_id(opts.client_id.clone())
        .scopes(vec![
            "openid",
            "profile",
            "email",
            "offline_access",
            "goauthentik.io/api",
        ])
        .poll_interval(Duration::from_secs(5))
        .generic_provider(GenericProviderConfig::new(
            urls.device_code_url,
            urls.token_url,
            "authentik".to_owned(),
        ))
        .max_attempts(12);

    let mut device_flow = DeviceFlow::new(Provider::Generic, config)?;

    let auth_response = device_flow.initialize().await?;

    let verification_uri = match auth_response.verification_uri_complete() {
        Some(vu) => vu,
        None => auth_response.verification_uri(),
    };
    callback(verification_uri.clone())?;

    log::debug!("Waiting for authentication...");
    let token_response = device_flow.poll_for_token().await?;

    let mut profile = Profile {
        authentik_url: opts.authentik_url.clone(),
        app_slug: opts.app_slug.clone(),
        client_id: opts.client_id.clone(),
        access_token: Some(token_response.access_token().to_owned()),
        refresh_token: None,
    };
    if let Some(token) = token_response.refresh_token() {
        profile.refresh_token = Some(token.to_owned())
    }
    Ok(profile)
}
