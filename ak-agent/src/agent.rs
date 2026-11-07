use std::sync::Arc;

use ak_platform::paths::xdg_config_path;
use ak_platform::storage::cfgmgr::ConfigManager;
use eyre::{Result, bail};
use waitgroup::WaitGroup;

use crate::config::{ConfigV1, ConfigV1Profile, Settings, policy::check_policy};
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
        check_policy()?;
        let cfg = ConfigManager::new(xdg_config_path("config.json")?).await?;
        let cc = Arc::clone(&cfg);
        Ok(Agent {
            cfg: cc,
            gtm: Arc::new(GlobalTokenManager::new(Arc::clone(&cfg)).await?),
        })
    }

    /// The user's settings with the admin policy enforced.
    pub async fn settings(&self) -> Settings {
        self.cfg.read().await.settings.effective()
    }

    /// Store a new profile, activate it if no profile is active yet and wait for its token manager.
    pub async fn setup_profile(&self, name: &str, profile: ConfigV1Profile) -> Result<()> {
        {
            let mut cfg = self.cfg.write().await;
            let rules = cfg.settings.effective().profile_management;
            rules.check_url(&profile.authentik_url)?;
            rules.check_dpop(profile.dpop_enabled())?;
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
            if !cfg.settings.effective().profile_management.allow_delete() {
                bail!("deleting profiles is disabled by policy");
            }
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
        let ssh_enabled = self.settings().await.ssh.enabled();

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
        if ssh_enabled {
            let w_ssh = wg.worker();
            tokio::spawn(async move {
                let ssh = match AgentSSHServer::new(Arc::clone(&shared)).await {
                    Ok(s) => s,
                    Err(e) => {
                        tracing::error!("failed to create ssh agent: {e:?}");
                        return;
                    }
                };
                if let Err(e) = ssh.start().await {
                    tracing::error!("failed to start ssh agent: {e:?}");
                }
                drop(w_ssh);
            });
        } else {
            tracing::info!("ssh-agent: disabled by settings");
        }
        wg.wait().await;
        Ok(())
    }
}
