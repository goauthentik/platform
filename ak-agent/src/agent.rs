use std::sync::Arc;

use ak_platform::paths::xdg_config_path;
use ak_platform::storage::cfgmgr::ConfigManager;
use eyre::Result;
use waitgroup::WaitGroup;

use crate::config::{ConfigV1, ConfigV1Profile};
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

    /// Store a new profile, activate it if no profile is active yet and wait for its token manager.
    pub async fn setup_profile(&self, name: &str, profile: ConfigV1Profile) -> Result<()> {
        {
            let mut cfg = self.cfg.write().await;
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
