use crate::state::StateStore;
use ak_meta::user_agent;
use ak_platform_keyring::KeyringStore;
use authentik_client::apis::configuration::Configuration;
use authentik_client::models::{AgentConfig, CurrentBrand, EnrollRequest};
use eyre::{Context, Result, bail, eyre};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Domain tokens are stored in the keyring under this service name, scoped
/// per-domain via the `user` field (the domain name).
fn keyring_service() -> String {
    ak_platform_keyring::service("sysd-domain-token")
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct DomainConfig {
    pub enabled: bool,
    pub authentik_url: String,
    pub domain: String,
    pub managed: bool,
    /// Only present in the JSON file if the keyring is unavailable.
    #[serde(rename = "token", default)]
    pub fallback_token: String,
    #[serde(skip)]
    pub token: String,
}

impl DomainConfig {
    fn file_name(&self) -> String {
        format!("{}.json", self.domain)
    }
}

#[derive(Debug)]
pub struct LoadedDomain {
    pub cfg: DomainConfig,
    pub api: Configuration,
    pub remote: Arc<RwLock<Option<AgentConfig>>>,
    // Supplies the flow_authentication slug used by interactive auth.
    pub brand: Arc<RwLock<Option<CurrentBrand>>>,
}

enum TokenFormat {
    Bearer,
    BearerAgent,
}

fn build_api_client(
    authentik_url: &str,
    token: &str,
    format: TokenFormat,
) -> Result<Configuration> {
    let header_value = match format {
        TokenFormat::Bearer => format!("Bearer {}", token,),
        TokenFormat::BearerAgent => format!("Bearer+agent {}", token,),
    };
    Ok(Configuration {
        base_path: format!("{}/api/v3", authentik_url.trim_end_matches('/')),
        bearer_access_token: None,
        user_agent: Some(user_agent()),
        client: reqwest_middleware::ClientBuilder::new(
            Client::builder()
                .default_headers(
                    [
                        (
                            reqwest::header::AUTHORIZATION,
                            reqwest::header::HeaderValue::from_str(&header_value)?,
                        ),
                        (
                            reqwest::header::HeaderName::from_str("X-AK-Platform-Version")?,
                            reqwest::header::HeaderValue::from_str(&ak_meta::version())?,
                        ),
                    ]
                    .into_iter()
                    .collect(),
                )
                .build()?,
        )
        .build(),
        ..Default::default()
    })
}

/// Rejects domain names that would let a saved/deleted file escape
/// `domain_dir` (e.g. via `..` or path separators) before it's ever joined
/// into a path — ported from Go's `ensurePathWithinDir` guard.
fn validate_domain_name(name: &str) -> Result<()> {
    if name.is_empty() || name.contains('/') || name.contains('\\') || name == "." || name == ".." {
        bail!("invalid domain name: {name}");
    }
    Ok(())
}

#[derive(Debug)]
pub struct DomainManager {
    domain_dir: String,
    state: Arc<StateStore>,
    domains: RwLock<Vec<Arc<LoadedDomain>>>,
}

impl DomainManager {
    pub async fn new(domain_dir: String, state: Arc<StateStore>) -> Result<Arc<Self>> {
        let dm = Arc::new(Self {
            domain_dir,
            state,
            domains: RwLock::new(Vec::new()),
        });
        dm.load_all().await?;
        Ok(dm)
    }

    /// Loads every `*.json` file in `domain_dir`, pulling each domain's token
    /// from the keyring (falling back to the in-file token if the keyring
    /// entry is missing), then loads any MDM-managed domain on top.
    pub async fn load_all(&self) -> Result<()> {
        tracing::info!("Loading domains...");
        let mut loaded = Vec::new();

        if !self.domain_dir.is_empty() {
            std::fs::create_dir_all(&self.domain_dir).ok();
            let entries = std::fs::read_dir(&self.domain_dir)
                .map_err(|e| eyre!("failed to read domain_dir {}: {e}", self.domain_dir))?;
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) != Some("json") {
                    continue;
                }
                let raw = std::fs::read_to_string(&path)?;
                let mut cfg: DomainConfig = serde_json::from_str(&raw)?;
                cfg.token = self.resolve_token(&cfg).await;

                // Pre-seed from the on-disk cache so components have a
                // last-known-good AgentConfig/brand before healthcheck_all's
                // network round-trip completes (offline/degraded-network
                // resilience, mirroring Go's bbolt-backed cache).
                let (remote, brand) = match self.state.domain_cache_get(&cfg.domain).await {
                    Ok(Some((cfg_json, brand_json, _))) => (
                        serde_json::from_str(&cfg_json).ok(),
                        serde_json::from_str(&brand_json).ok(),
                    ),
                    Ok(None) => (None, None),
                    Err(e) => {
                        tracing::warn!(
                            domain = %cfg.domain,
                            "failed to load cached domain config: {e:?}"
                        );
                        (None, None)
                    }
                };

                loaded.push(Arc::new(LoadedDomain {
                    api: build_api_client(
                        &cfg.authentik_url,
                        &cfg.token,
                        TokenFormat::BearerAgent,
                    )?,
                    remote: Arc::new(RwLock::new(remote)),
                    brand: Arc::new(RwLock::new(brand)),
                    cfg,
                }));
            }
        }

        tracing::info!("Loaded {} domains", loaded.len());
        *self.domains.write().await = loaded;

        if let Err(e) = self.load_managed().await {
            tracing::warn!("failed to load managed domain config: {e:?}");
        }
        Ok(())
    }

    async fn resolve_token(&self, cfg: &DomainConfig) -> String {
        match ak_platform_keyring::store()
            .get(
                &keyring_service(),
                &cfg.domain,
                ak_platform_keyring::Accessibility::Always,
            )
            .await
        {
            Ok(token) => token,
            Err(e) => {
                tracing::warn!(
                    "failed load domain token from keyring, falling back to file: {e:?}"
                );
                cfg.fallback_token.clone()
            }
        }
    }

    pub async fn domains(&self) -> Vec<Arc<LoadedDomain>> {
        self.domains.read().await.clone()
    }

    pub async fn min_refresh_interval(&self) -> Option<u64> {
        // Smallest refresh_interval (seconds) across domains that have a
        // loaded remote config; fall back to the default when none is available.
        let mut refresh_interval: Option<u64> = None;
        for d in self.domains().await {
            if let Some(remote) = d.remote.read().await.as_ref()
                && remote.refresh_interval > 0
            {
                let secs = remote.refresh_interval as u64;
                refresh_interval = Some(refresh_interval.map_or(secs, |m| m.min(secs)));
            }
        }
        refresh_interval
    }

    /// First enabled domain that has a token — mirrors Go's `dom[0]` shortcut
    /// for single-tenant components (ping, auth, directory, device). Do not
    /// invent smarter "current domain" selection here.
    ///
    /// The token check matters because `load_managed` adds an MDM-managed
    /// domain alongside any user-enrolled ones. If its enrollment produced no
    /// token, it is still `enabled`, and returning it means every request goes
    /// out as `Bearer+agent ` and comes back 403 "Authentication credentials
    /// were not provided" — while a perfectly good domain sits further down the
    /// list.
    pub async fn active(&self) -> Result<Arc<LoadedDomain>> {
        let selected = self
            .domains
            .read()
            .await
            .iter()
            .find(|d| d.cfg.enabled && !d.cfg.token.is_empty())
            .cloned()
            .ok_or_else(|| eyre!("no enabled domain with a token configured"))?;
        tracing::debug!(domain = %selected.cfg.domain, "selected active domain");
        Ok(selected)
    }

    pub async fn save_domain(&self, cfg: DomainConfig) -> Result<()> {
        validate_domain_name(&cfg.domain)?;
        let mut on_disk = cfg.clone();

        match ak_platform_keyring::store()
            .set(
                &keyring_service(),
                &cfg.domain,
                ak_platform_keyring::Accessibility::Always,
                cfg.token.clone(),
            )
            .await
        {
            // A write that cannot be read back is no use to us, so confirm the
            // value is retrievable before dropping the on-disk copy. On macOS
            // the write succeeds from a launchd daemon but the read fails with
            // errSecInteractionNotAllowed, because retrieving the item wants an
            // ACL prompt and there is no UI session to show one. Clearing the
            // fallback on the strength of the write alone leaves the token
            // somewhere sysd can never reach, and every subsequent request goes
            // out as `Bearer+agent ` and comes back 403.
            Ok(_) => match ak_platform_keyring::store()
                .get(
                    &keyring_service(),
                    &cfg.domain,
                    ak_platform_keyring::Accessibility::Always,
                )
                .await
            {
                Ok(stored) if stored == cfg.token => on_disk.fallback_token = String::new(),
                Ok(_) => {
                    on_disk.fallback_token = cfg.token.clone();
                    tracing::warn!(
                        "keyring returned a different token than was written, keeping file fallback"
                    );
                }
                Err(e) => {
                    on_disk.fallback_token = cfg.token.clone();
                    tracing::warn!(
                        "saved domain token to keyring but could not read it back ({e:?}), keeping file fallback"
                    );
                }
            },
            Err(e) => {
                on_disk.fallback_token = cfg.token.clone();
                tracing::warn!(
                    "failed to save domain token to keyring, falling back to file: {e:?}"
                );
            }
        }

        let path = std::path::Path::new(&self.domain_dir).join(cfg.file_name());
        std::fs::create_dir_all(&self.domain_dir)?;

        let json = serde_json::to_string_pretty(&on_disk)?;
        std::fs::write(&path, json)?;

        let loaded = Arc::new(LoadedDomain {
            api: build_api_client(&cfg.authentik_url, &cfg.token, TokenFormat::BearerAgent)?,
            remote: Arc::new(RwLock::new(None)),
            brand: Arc::new(RwLock::new(None)),
            cfg,
        });
        let mut domains = self.domains.write().await;
        domains.retain(|d| d.cfg.domain != loaded.cfg.domain);
        domains.push(loaded);
        Ok(())
    }

    pub async fn delete_domain(&self, name: &str) -> Result<()> {
        validate_domain_name(name)?;
        let path = std::path::Path::new(&self.domain_dir).join(format!("{name}.json"));
        if path.exists() {
            std::fs::remove_file(&path)?;
        }
        if let Err(e) = ak_platform_keyring::store()
            .delete(
                &keyring_service(),
                name,
                ak_platform_keyring::Accessibility::Always,
            )
            .await
        {
            tracing::warn!("failed to delete domain token from keyring: {e:?}");
        }
        self.domains.write().await.retain(|d| d.cfg.domain != name);
        Ok(())
    }

    /// Exchanges a one-time registration token for a permanent device token.
    /// Does not save the resulting domain — callers must call
    /// `save_domain` explicitly, mirroring Go's `Enroll()`/`SaveDomain()` split.
    pub async fn enroll(
        &self,
        domain: String,
        authentik_url: String,
        one_time_token: String,
    ) -> Result<DomainConfig> {
        let serial = ak_platform_facts::serial().context("failed to get serial")?;
        let hostname = ak_platform_facts::hostname();
        let api = build_api_client(&authentik_url, &one_time_token, TokenFormat::Bearer)
            .context("failed to get API client")?;
        let res = authentik_client::apis::endpoints_api::endpoints_agents_connectors_enroll_create(
            &api,
            EnrollRequest::new(serial, hostname),
        )
        .await
        .map_err(|e| eyre!("enrollment failed: {e}"))?;

        Ok(DomainConfig {
            enabled: true,
            authentik_url,
            domain,
            managed: false,
            fallback_token: String::new(),
            token: res.token,
        })
    }

    /// Verifies connectivity for a domain, mirroring Go's `Test()`.
    pub async fn test(&self, cfg: &DomainConfig) -> Result<AgentConfig> {
        let api = build_api_client(&cfg.authentik_url, &cfg.token, TokenFormat::BearerAgent)?;
        let remote = authentik_client::apis::endpoints_api::endpoints_agents_connectors_agent_config_retrieve(&api)
            .await
            .map_err(|e| eyre!("connectivity test failed: {e}"))?;
        Ok(remote)
    }

    /// Tests every loaded domain, refreshing `remote`/`brand` and persisting
    /// to the on-disk `domain_cache`; any that fail are logged and left
    /// enabled (mirroring the lack of hard-disable behavior actually
    /// verified in this pass — flagged for confirmation against
    /// `SystemAgent.DomainCheck()`).
    pub async fn healthcheck_all(&self) {
        let domains = self.domains().await;
        for d in domains {
            if let Err(e) = self.fetch_remote_config(&d.cfg.domain).await {
                tracing::warn!(domain = d.cfg.domain, "domain healthcheck failed: {e:?}");
            }
        }
    }

    /// Refreshes a domain's cached `AgentConfig`, writing it into the
    /// `domain_cache` state table alongside the in-memory copy.
    pub async fn fetch_remote_config(&self, domain_name: &str) -> Result<()> {
        let domains = self.domains().await;
        let Some(d) = domains.iter().find(|d| d.cfg.domain == domain_name) else {
            bail!("domain not found: {domain_name}");
        };
        let remote = self.test(&d.cfg).await?;
        let cfg_json = serde_json::to_string(&remote)?;

        let brand = authentik_client::apis::core_api::core_brands_current_retrieve(&d.api)
            .await
            .map_err(|e| eyre!("failed to fetch current brand: {e}"))?;
        let brand_json = serde_json::to_string(&brand)?;

        self.state
            .domain_cache_set(
                domain_name,
                &cfg_json,
                &brand_json,
                chrono::Utc::now().timestamp(),
            )
            .await
            .context("failed to persist domain cache")?;
        *d.remote.write().await = Some(remote);
        *d.brand.write().await = Some(brand);
        Ok(())
    }

    /// Loads (or re-enrolls, or removes) the MDM-managed domain. See
    /// `cfg::managed` for the platform-specific config source.
    pub async fn load_managed(&self) -> Result<()> {
        let Some(managed) = crate::cfg::managed::load_managed_config()? else {
            return Ok(());
        };

        const MANAGED_DOMAIN_NAME: &str = "ak-mdm-managed";
        let existing = self
            .domains()
            .await
            .into_iter()
            .find(|d| d.cfg.domain == MANAGED_DOMAIN_NAME);

        if let Some(existing) = existing {
            if existing
                .cfg
                .authentik_url
                .eq_ignore_ascii_case(&managed.url)
            {
                tracing::debug!("resumed existing managed domain");
                return Ok(());
            }
            if let Err(e) = self.delete_domain(MANAGED_DOMAIN_NAME).await {
                tracing::warn!("failed to delete old managed domain: {e:?}");
            }
        }

        let cfg = self
            .enroll(
                MANAGED_DOMAIN_NAME.to_string(),
                managed.url,
                managed.registration_token,
            )
            .await
            .map_err(|e| eyre!("failed to enroll managed domain: {e}"))?;
        let mut cfg = cfg;
        cfg.managed = true;
        self.save_domain(cfg).await?;
        Ok(())
    }
}

#[cfg(any(test, debug_assertions))]
#[allow(clippy::unwrap_used)]
pub mod testutils {
    use std::sync::Arc;

    use crate::{cfg::domain::DomainManager, state::StateStore};
    use tempfile::TempDir;

    pub async fn test_manager(state: Arc<StateStore>) -> Arc<DomainManager> {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("ak-sysd-domains");
        DomainManager::new(path.to_str().unwrap().to_string(), state)
            .await
            .unwrap()
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use crate::state::testutils::test_store;
    use authentik_client::models::{AgentConfig, CurrentBrand};
    use std::sync::Arc;
    use tempfile::TempDir;

    /// Regression test for the domain_cache write/read round trip: a value
    /// written via `domain_cache_set` must come back out through
    /// `domain_cache_get` and pre-seed `LoadedDomain.remote`/`.brand` on the
    /// next `load_all`, without ever touching the network.
    #[tokio::test]
    async fn load_all_pre_seeds_remote_and_brand_from_domain_cache() {
        let state = Arc::new(test_store());
        let dir = TempDir::new().unwrap();

        let cfg = DomainConfig {
            enabled: true,
            authentik_url: "https://authentik.example".to_string(),
            domain: "cached-domain".to_string(),
            managed: false,
            fallback_token: "atoken".to_string(),
            token: String::new(),
        };
        std::fs::write(
            dir.path().join(cfg.file_name()),
            serde_json::to_string(&cfg).unwrap(),
        )
        .unwrap();

        let remote = AgentConfig {
            device_id: "device-123".to_string(),
            ..Default::default()
        };
        let brand = CurrentBrand {
            flow_authentication: Some("default-authentication-flow".to_string()),
            ..Default::default()
        };

        state
            .domain_cache_set(
                &cfg.domain,
                &serde_json::to_string(&remote).unwrap(),
                &serde_json::to_string(&brand).unwrap(),
                1234,
            )
            .await
            .unwrap();

        let manager = DomainManager::new(dir.path().to_str().unwrap().to_string(), state)
            .await
            .unwrap();

        let domains = manager.domains().await;
        let loaded = domains
            .iter()
            .find(|d| d.cfg.domain == "cached-domain")
            .expect("domain should be loaded");

        let loaded_remote = loaded
            .remote
            .read()
            .await
            .clone()
            .expect("remote should be pre-seeded from cache");
        assert_eq!(loaded_remote.device_id, "device-123");

        let loaded_brand = loaded
            .brand
            .read()
            .await
            .clone()
            .expect("brand should be pre-seeded from cache");
        assert_eq!(
            loaded_brand.flow_authentication,
            Some("default-authentication-flow".to_string())
        );
    }
}
