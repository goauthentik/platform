use std::{collections::HashMap, fmt::Debug};

use ak_meta::user_agent;
use ak_platform::dpop::{DpopKeyPair, DpopSigner};
use ak_platform::log::LevelFilter;
use ak_platform::log::set_log_level;
use ak_platform::paths::DEFAULT_PROFILE;
use ak_platform::storage::cfgmgr::schema::Config;
use ak_platform_keyring;
use ak_platform_keyring::KeyringStore;
use ak_platform_keyring::hardware::HardwareSigningKey;
use authentik_client::apis::configuration::Configuration;
use eyre::Result;
use reqwest::Client;
use serde::{Deserialize, Serialize};

/// `hardware-enclave` app identity for DPoP keys — grouped under the same
/// dev/prod credential namespace as the other keyring-stored secrets.
pub(crate) fn dpop_hardware_app_name() -> String {
    ak_platform_keyring::service("dpop")
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DpopKeyBackend {
    #[default]
    None,
    Software,
    Hardware,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigV1 {
    pub debug: bool,
    #[serde(default)]
    pub active_profile: String,
    pub profiles: HashMap<String, ConfigV1Profile>,
}

impl Default for ConfigV1 {
    fn default() -> Self {
        Self {
            debug: false,
            active_profile: DEFAULT_PROFILE.to_string(),
            profiles: Default::default(),
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ConfigV1Profile {
    pub authentik_url: String,
    pub app_slug: String,
    pub client_id: String,

    // Fallback if keyring isn't available
    #[serde(rename = "access_token")]
    pub fallback_access_token: String,
    #[serde(rename = "refresh_token")]
    pub fallback_refresh_token: String,
    // Empty string unless dpop_key_backend == Software.
    #[serde(rename = "dpop_private_key", default)]
    pub fallback_dpop_private_key: String,
    // Which key backend (if any) this profile's DPoP proofs are signed with.
    // Plain, non-secret — stored directly in the config JSON, not the keyring.
    #[serde(default)]
    pub dpop_key_backend: DpopKeyBackend,

    // Not saved to JSON, loaded from keychain
    #[serde(skip)]
    _access_token: String,
    #[serde(skip)]
    _refresh_token: String,
    // PKCS#8 PEM; only meaningful when dpop_key_backend == Software. A
    // Hardware-backed key has no material to store here — the enclave itself
    // durably persists it, addressable again via (app_name, profile_name).
    #[serde(skip)]
    _dpop_private_key: String,

    #[serde(skip)]
    _http_client: Option<Client>,
}

impl Debug for ConfigV1Profile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ConfigV1Profile")
            .field("authentik_url", &self.authentik_url)
            .field("app_slug", &self.app_slug)
            .field("client_id", &self.client_id)
            .field("fallback_access_token", &self.fallback_access_token.len())
            .field("fallback_refresh_token", &self.fallback_refresh_token.len())
            .field(
                "fallback_dpop_private_key",
                &self.fallback_dpop_private_key.len(),
            )
            .field("dpop_key_backend", &self.dpop_key_backend)
            .field("_access_token", &self._access_token.len())
            .field("_refresh_token", &self._refresh_token.len())
            .field("_dpop_private_key", &self._dpop_private_key.len())
            .field("_http_client", &self._http_client)
            .finish()
    }
}

impl ConfigV1Profile {
    /// Builds a fresh, non-DPoP profile. Profiles that use a DPoP key are
    /// created ahead of time by the `PrepareDpopKey` RPC (see
    /// `grpc/agent_ctrl.rs::prepare_dpop_key`); `Setup` only fills in tokens
    /// onto that existing entry, preserving its `dpop_key_backend`.
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
            fallback_dpop_private_key: "".to_string(),
            dpop_key_backend: DpopKeyBackend::None,
            _access_token: access_token,
            _refresh_token: refresh_token,
            _dpop_private_key: String::new(),
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

    /// Sets the profile's stored DPoP private-key material. Only meaningful
    /// for `DpopKeyBackend::Software` — pass an empty string for `None`/`Hardware`.
    pub fn set_dpop_private_key<T: ToString>(&mut self, t: T) {
        self._dpop_private_key = t.to_string()
    }

    /// Whether this profile has a DPoP key bound to it.
    pub fn dpop_enabled(&self) -> bool {
        self.dpop_key_backend != DpopKeyBackend::None
    }

    /// The profile's DPoP signer, if it has one. `profile_name` is the
    /// caller's own key into `ConfigV1::profiles` (not stored redundantly on
    /// `ConfigV1Profile` itself) — for a hardware-backed profile it doubles as
    /// the enclave key label, so it must be the exact same name used when the
    /// key was created via `PrepareDpopKey`.
    pub fn dpop_signer(&self, profile_name: &str) -> Result<Option<DpopSigner>> {
        match self.dpop_key_backend {
            DpopKeyBackend::None => Ok(None),
            DpopKeyBackend::Software => Ok(Some(DpopSigner::Software(
                DpopKeyPair::from_pkcs8_pem(&self._dpop_private_key)?,
            ))),
            DpopKeyBackend::Hardware => Ok(Some(DpopSigner::Hardware(
                HardwareSigningKey::open_or_generate(&dpop_hardware_app_name(), profile_name)?,
            ))),
        }
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
        set_log_level(match self.debug {
            true => LevelFilter::Trace,
            false => LevelFilter::Trace,
        });
        for (key, val) in self.profiles.iter_mut() {
            tracing::debug!(profile = key, "Getting access token for profile");
            match ak_platform_keyring::store()
                .get(
                    &ak_platform_keyring::service("access_token"),
                    key,
                    ak_platform_keyring::Accessibility::User,
                )
                .await
            {
                Ok(v) => val._access_token = v,
                Err(ak_platform_keyring::KeyringError::NotAvailable())
                | Err(ak_platform_keyring::KeyringError::NotFound()) => {
                    val._access_token = val.fallback_access_token.clone()
                }
                Err(e) => return Err(e.into()),
            }
            tracing::debug!(profile = key, "Getting refresh token for profile");
            match ak_platform_keyring::store()
                .get(
                    &ak_platform_keyring::service("refresh_token"),
                    key,
                    ak_platform_keyring::Accessibility::User,
                )
                .await
            {
                Ok(v) => val._refresh_token = v,
                Err(ak_platform_keyring::KeyringError::NotAvailable())
                | Err(ak_platform_keyring::KeyringError::NotFound()) => {
                    val._refresh_token = val.fallback_refresh_token.clone()
                }
                Err(e) => return Err(e.into()),
            }
            tracing::debug!(profile = key, "Getting DPoP private key for profile");
            match ak_platform_keyring::store()
                .get(
                    &ak_platform_keyring::service("dpop_private_key"),
                    key,
                    ak_platform_keyring::Accessibility::User,
                )
                .await
            {
                Ok(v) => val._dpop_private_key = v,
                Err(ak_platform_keyring::KeyringError::NotAvailable())
                | Err(ak_platform_keyring::KeyringError::NotFound()) => {
                    val._dpop_private_key = val.fallback_dpop_private_key.clone()
                }
                Err(e) => return Err(e.into()),
            }
        }
        Ok(())
    }

    async fn pre_save(&mut self) -> Result<()> {
        for (key, val) in self.profiles.iter_mut() {
            match ak_platform_keyring::store()
                .set(
                    &ak_platform_keyring::service("access_token"),
                    key,
                    ak_platform_keyring::Accessibility::User,
                    val._access_token.clone(),
                )
                .await
            {
                Ok(_) => {}
                Err(ak_platform_keyring::KeyringError::NotAvailable()) => {
                    val.fallback_access_token = val._access_token.clone();
                }
                Err(e) => return Err(e.into()),
            };
            match ak_platform_keyring::store()
                .set(
                    &ak_platform_keyring::service("refresh_token"),
                    key,
                    ak_platform_keyring::Accessibility::User,
                    val._refresh_token.clone(),
                )
                .await
            {
                Ok(_) => {}
                Err(ak_platform_keyring::KeyringError::NotAvailable()) => {
                    val.fallback_refresh_token = val._refresh_token.clone();
                }
                Err(e) => return Err(e.into()),
            };
            match ak_platform_keyring::store()
                .set(
                    &ak_platform_keyring::service("dpop_private_key"),
                    key,
                    ak_platform_keyring::Accessibility::User,
                    val._dpop_private_key.clone(),
                )
                .await
            {
                Ok(_) => {}
                Err(ak_platform_keyring::KeyringError::NotAvailable()) => {
                    val.fallback_dpop_private_key = val._dpop_private_key.clone();
                }
                Err(e) => return Err(e.into()),
            };
        }
        Ok(())
    }
}
