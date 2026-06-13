use std::collections::HashMap;

use ak_platform::prelude::*;
use ak_platform::{keyring, storage::cfgmgr::schema::Config};
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
    pub _access_token: String,
    #[serde(skip)]
    pub _refresh_token: String,

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
            fallback_access_token: access_token.clone(),
            fallback_refresh_token: refresh_token.clone(),
            _access_token: access_token,
            _refresh_token: refresh_token,
            _http_client: None,
        }
    }

    pub fn http_client(mut self) -> Client {
        match self._http_client {
            Some(c) => c,
            None => {
                let c = reqwest::Client::new();
                self._http_client = Some(c.clone());
                c
            }
        }
    }
}

impl Config for ConfigV1 {
    async fn post_load(&mut self) -> Result<()> {
        for (key, val) in self.profiles.iter_mut() {
            log::debug!("Getting access token for profile: {key}");
            match keyring::get(
                &keyring::service("access_token"),
                key,
                keyring::Accessibility::User,
            )
            .await
            {
                Ok(v) => val._access_token = v,
                Err(keyring::KeyringError::NotFound()) => {
                    val._access_token = val.fallback_access_token.clone()
                }
                Err(e) => return Err(e.into()),
            }
            log::debug!("Getting refresh token for profile: {key}");
            match keyring::get(
                &keyring::service("refresh_token"),
                key,
                keyring::Accessibility::User,
            )
            .await
            {
                Ok(v) => val._refresh_token = v,
                Err(keyring::KeyringError::NotFound()) => {
                    val._refresh_token = val.fallback_refresh_token.clone()
                }
                Err(e) => return Err(e.into()),
            }
        }
        Ok(())
    }
    async fn pre_save(&self) -> Result<()> {
        for (key, val) in self.profiles.iter() {
            keyring::set(
                &keyring::service("access_token"),
                key,
                keyring::Accessibility::User,
                val._access_token.clone(),
            )
            .await?;
            keyring::set(
                &keyring::service("refresh_token"),
                key,
                keyring::Accessibility::User,
                val._refresh_token.clone(),
            )
            .await?;
        }
        Ok(())
    }
}
