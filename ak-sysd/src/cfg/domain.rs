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
                loaded.push(Arc::new(LoadedDomain {
                    api: build_api_client(
                        &cfg.authentik_url,
                        &cfg.token,
                        TokenFormat::BearerAgent,
                    )?,
                    remote: Arc::new(RwLock::new(None)),
                    brand: Arc::new(RwLock::new(None)),
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

    /// First enabled domain — mirrors Go's `dom[0]` shortcut for
    /// single-tenant components (ping, auth, directory, device). Do not
    /// invent smarter "current domain" selection here.
    pub async fn active(&self) -> Result<Arc<LoadedDomain>> {
        self.domains
            .read()
            .await
            .iter()
            .find(|d| d.cfg.enabled)
            .cloned()
            .ok_or_else(|| eyre!("no enabled domain configured"))
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
            Ok(_) => on_disk.fallback_token = String::new(),
            Err(e) => {
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

    /// Tests every loaded domain; any that fail are logged and left enabled
    /// (mirroring the lack of hard-disable behavior actually verified in
    /// this pass — flagged for confirmation against `SystemAgent.DomainCheck()`).
    pub async fn healthcheck_all(&self) {
        let domains = self.domains().await;
        for d in domains {
            match self.test(&d.cfg).await {
                Ok(remote) => {
                    *d.remote.write().await = Some(remote);
                }
                Err(e) => {
                    tracing::warn!(domain = d.cfg.domain, "domain healthcheck failed: {e:?}");
                }
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
            .await?;
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
