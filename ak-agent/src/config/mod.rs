use std::collections::HashMap;

use ak_meta::user_agent;
use ak_platform::prelude::*;
use ak_platform::storage::cfgmgr::schema::Config;
use ak_platform_keyring;
use authentik_client::apis::configuration::Configuration;
use reqwest::Client;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct ConfigV1 {
    pub debug: bool,
    pub profiles: HashMap<String, ConfigV1Profile>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigV1Profile {
    pub authentik_url: String,
    pub app_slug: String,
    pub client_id: String,

    // Fallback if keyring isn't available
    #[serde(rename = "access_token")]
    pub fallback_access_token: String,
    #[serde(rename = "refresh_token")]
    pub fallback_refresh_token: String,

    // Not saved to JSON, loaded from keychain
    #[serde(skip)]
    _access_token: String,
    #[serde(skip)]
    _refresh_token: String,

    #[serde(skip)]
    _http_client: Option<Client>,
}

impl ConfigV1Profile {
    pub fn from_tokens(
        authentik_url: String,
        app_slug: String,
        client_id: String,
        access_token: String,
        refresh_token: String,
    ) -> Self {
        ConfigV1Profile {
            authentik_url,
            app_slug,
            client_id,
            fallback_access_token: "".to_string(),
            fallback_refresh_token: "".to_string(),
            _access_token: access_token,
            _refresh_token: refresh_token,
            _http_client: None,
        }
    }

    pub fn access_token(&self) -> String {
        self._access_token.clone()
    }

    pub fn refresh_token(&self) -> String {
        self._refresh_token.clone()
    }

    pub fn set_access_token<T: ToString>(&mut self, t: T) {
        self._access_token = t.to_string()
    }

    pub fn set_refresh_token<T: ToString>(&mut self, t: T) {
        self._refresh_token = t.to_string()
    }

    pub fn http_client(mut self) -> Client {
        match self._http_client {
            Some(c) => c,
            None => {
                let c = Client::new();
                self._http_client = Some(c.clone());
                c
            }
        }
    }

    // TEMP, the authentik-client crate currently incorrectly drops the auth for certain
    // endpoint-related endpoints, thus we inject it as a header in reqwest
    pub fn authenticated_http_client(self) -> Result<Client> {
        let c = Client::builder()
            .default_headers(
                [(
                    reqwest::header::AUTHORIZATION,
                    reqwest::header::HeaderValue::from_str(&format!(
                        "Bearer {}",
                        self.access_token()
                    ))?,
                )]
                .into_iter()
                .collect(),
            )
            .build()?;
        Ok(c)
    }

    pub fn api_config(self) -> Result<Configuration> {
        Ok(Configuration {
            base_path: format!("{}/api/v3", self.authentik_url.clone()),
            bearer_access_token: Some(self.access_token()),
            user_agent: Some(user_agent()),
            client: reqwest_middleware::ClientBuilder::new(self.authenticated_http_client()?)
                .build(),
            basic_auth: None,
            oauth_access_token: None,
            api_key: None,
        })
    }
}

impl Config for ConfigV1 {
    async fn post_load(&mut self) -> Result<()> {
        for (key, val) in self.profiles.iter_mut() {
            log::debug!("Getting access token for profile: {key}");
            match ak_platform_keyring::get(
                &ak_platform_keyring::service("access_token"),
                key,
                ak_platform_keyring::Accessibility::User,
            )
            .await
            {
                Ok(v) => val._access_token = v,
                Err(ak_platform_keyring::KeyringError::NotFound()) => {
                    val._access_token = val.fallback_access_token.clone()
                }
                Err(e) => return Err(e.into()),
            }
            log::debug!("Getting refresh token for profile: {key}");
            match ak_platform_keyring::get(
                &ak_platform_keyring::service("refresh_token"),
                key,
                ak_platform_keyring::Accessibility::User,
            )
            .await
            {
                Ok(v) => val._refresh_token = v,
                Err(ak_platform_keyring::KeyringError::NotFound()) => {
                    val._refresh_token = val.fallback_refresh_token.clone()
                }
                Err(e) => return Err(e.into()),
            }
        }
        Ok(())
    }

    async fn pre_save(&self) -> Result<()> {
        for (key, val) in self.profiles.iter() {
            ak_platform_keyring::set(
                &ak_platform_keyring::service("access_token"),
                key,
                ak_platform_keyring::Accessibility::User,
                val._access_token.clone(),
            )
            .await?;
            ak_platform_keyring::set(
                &ak_platform_keyring::service("refresh_token"),
                key,
                ak_platform_keyring::Accessibility::User,
                val._refresh_token.clone(),
            )
            .await?;
        }
        Ok(())
    }
}
