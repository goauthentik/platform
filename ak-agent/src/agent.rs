use std::sync::Arc;

use ak_platform::dpop::{DpopKeyPair, DpopSigner};
use ak_platform::paths::xdg_config_path;
use ak_platform::storage::cfgmgr::ConfigManager;
use ak_platform_keyring::hardware::{HardwareKeyError, HardwareSigningKey};
use eyre::{Result, bail, eyre};
use waitgroup::WaitGroup;

use crate::config::{ConfigV1, ConfigV1Profile, DpopKeyBackend, dpop_hardware_app_name};
use crate::grpc::AgentGRPCServer;
use crate::ssh::AgentSSHServer;
use crate::token::global::GlobalTokenManager;

#[derive(Clone)]
pub struct Agent {
    pub cfg: Arc<ConfigManager<ConfigV1>>,
    pub gtm: Arc<GlobalTokenManager>,
}

impl Agent {
    pub async fn new() -> Result<Self> {
        let cfg = ConfigManager::new(xdg_config_path("config.json")?).await?;
        let cc = Arc::clone(&cfg);
        Ok(Agent {
            cfg: cc,
            gtm: Arc::new(GlobalTokenManager::new(Arc::clone(&cfg)).await?),
        })
    }

    /// Establish (generating if needed) the DPoP key for a profile before the
    /// device flow starts, since its thumbprint must ride the initial
    /// device-authorization request. Creates the profile if it doesn't exist
    /// yet. Returns the signer and whether it is hardware-backed.
    pub async fn prepare_dpop_key(
        &self,
        name: &str,
        authentik_url: String,
        app_slug: String,
        client_id: String,
    ) -> Result<(DpopSigner, bool)> {
        let software = || -> Result<_> {
            let kp = DpopKeyPair::generate();
            let pem = kp.to_pkcs8_pem()?;
            Ok((
                DpopSigner::Software(kp),
                DpopKeyBackend::Software,
                pem,
                false,
            ))
        };
        let (signer, dpop_key_backend, dpop_private_key, hardware_backed) =
            match HardwareSigningKey::open_or_generate(&dpop_hardware_app_name(), name) {
                Ok(hw) => (
                    DpopSigner::Hardware(hw),
                    DpopKeyBackend::Hardware,
                    String::new(),
                    true,
                ),
                Err(HardwareKeyError::NotAvailable) => {
                    tracing::info!(
                        profile = name,
                        "no hardware key storage available on this device, using a software DPoP key"
                    );
                    software()?
                }
                Err(HardwareKeyError::Other(e)) => {
                    tracing::warn!(
                        profile = name,
                        error = %e,
                        "hardware DPoP key generation failed, falling back to a software key"
                    );
                    software()?
                }
            };

        {
            let mut cfg = self.cfg.write().await;
            let profile = cfg.profiles.entry(name.to_owned()).or_insert_with(|| {
                ConfigV1Profile::from_tokens(
                    authentik_url.clone(),
                    app_slug.clone(),
                    client_id.clone(),
                    String::new(),
                    String::new(),
                )
            });
            profile.authentik_url = authentik_url;
            profile.app_slug = app_slug;
            profile.client_id = client_id;
            profile.dpop_key_backend = dpop_key_backend;
            profile.set_dpop_private_key(dpop_private_key);
        }
        self.cfg.save().await?;
        tracing::info!(
            profile = name,
            hardware_backed,
            "prepared DPoP key for profile"
        );
        Ok((signer, hardware_backed))
    }

    /// Store a new profile, activate it if no profile is active yet and wait for its token manager.
    /// With `dpop_bound`, the key from a prior [`Self::prepare_dpop_key`] is carried over;
    /// otherwise the profile is stored without a DPoP key.
    pub async fn setup_profile(
        &self,
        name: &str,
        mut profile: ConfigV1Profile,
        dpop_bound: bool,
    ) -> Result<()> {
        {
            let mut cfg = self.cfg.write().await;
            if dpop_bound {
                let prepared = cfg
                    .profiles
                    .get(name)
                    .filter(|p| p.dpop_enabled())
                    .ok_or_else(|| eyre!("no DPoP key prepared for profile '{name}'"))?;
                profile.dpop_key_backend = prepared.dpop_key_backend;
                profile.set_dpop_private_key(prepared.dpop_private_key());
            }
            cfg.profiles.insert(name.to_owned(), profile);
            if cfg.active_profile.is_empty() {
                cfg.active_profile = name.to_owned();
            }
        }
        self.cfg.save().await?;
        self.gtm.wait_for_profile(name).await;
        tracing::info!(profile = name, "setup new profile");
        Ok(())
    }

    /// Remove a profile. If it was active, another remaining profile (if any) becomes active.
    pub async fn delete_profile(&self, name: &str) -> Result<()> {
        {
            let mut cfg = self.cfg.write().await;
            if cfg.profiles.remove(name).is_none() {
                bail!("profile '{name}' not found");
            }
            if cfg.active_profile == name {
                cfg.active_profile = cfg.profiles.keys().next().cloned().unwrap_or_default();
            }
        }
        self.cfg.save().await?;
        tracing::info!(profile = name, "deleted profile");
        Ok(())
    }

    pub async fn start(self) -> Result<()> {
        let wg = WaitGroup::new();

        let w_grpc = wg.worker();
        let w_ssh = wg.worker();

        let shared = Arc::new(self);
        let shared_grpc = Arc::clone(&shared);

        tokio::spawn(async move {
            let grpc = match AgentGRPCServer::new(shared_grpc).await {
                Ok(grpc) => grpc,
                Err(e) => {
                    tracing::error!("Failed to start grpc server: {e:?}");
                    return;
                }
            };
            match grpc.start().await {
                Ok(_) => (),
                Err(e) => {
                    tracing::error!("Failed to start grpc server: {e:?}");
                }
            };
            drop(w_grpc);
        });
        tokio::spawn(async move {
            let ssh = match AgentSSHServer::new(Arc::clone(&shared)).await {
                Ok(s) => s,
                Err(e) => {
                    tracing::error!("failed to create ssh agent: {e:?}");
                    return;
                }
            };
            match ssh.start().await {
                Ok(()) => (),
                Err(e) => {
                    tracing::error!("failed to start ssh agent: {e:?}");
                }
            };
            drop(w_ssh);
        });
        wg.wait().await;
        Ok(())
    }
}
