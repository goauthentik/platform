use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use tokio::sync::RwLock;

use ak_platform::prelude::*;
use ak_platform::storage::cfgmgr::ConfigManager;

use crate::config::ConfigV1;
use crate::token::profile::ProfileTokenManager;

static GLOBAL_CREATED: AtomicBool = AtomicBool::new(false);

pub struct GlobalTokenManager {
    cfg: Arc<ConfigManager<ConfigV1>>,
    managers: Arc<RwLock<HashMap<String, Arc<ProfileTokenManager>>>>,
}

impl GlobalTokenManager {
    pub async fn new(cfg: Arc<ConfigManager<ConfigV1>>) -> Result<Self> {
        if GLOBAL_CREATED.swap(true, Ordering::SeqCst) {
            return Err(Box::from("only a single global token manager can be used"));
        }
        let gtm = GlobalTokenManager {
            cfg: Arc::clone(&cfg),
            managers: Arc::new(RwLock::new(HashMap::new())),
        };
        gtm.start().await;
        Ok(gtm)
    }

    pub async fn start(&self) {
        {
            let config = self.cfg.read().await;
            let mut managers = self.managers.write().await;
            for name in config.profiles.keys() {
                match ProfileTokenManager::new_verified(name.clone(), Arc::clone(&self.cfg)).await {
                    Ok(m) => {
                        managers.insert(name.clone(), Arc::new(m));
                    }
                    Err(e) => {
                        log::warn!("failed to create manager for profile '{name}': {e:?}");
                    }
                }
            }
        }

        let cfg_bg = Arc::clone(&self.cfg);
        let managers_bg = Arc::clone(&self.managers);
        tokio::spawn(async move {
            Self::watch_config_changes(cfg_bg, managers_bg).await;
        });
    }

    async fn watch_config_changes(
        cfg: Arc<ConfigManager<ConfigV1>>,
        managers: Arc<RwLock<HashMap<String, Arc<ProfileTokenManager>>>>,
    ) {
        let notify = cfg.on_reload();
        loop {
            notify.notified().await;

            let current: HashSet<String> = cfg.read().await.profiles.keys().cloned().collect();
            let known: HashSet<String> = managers.read().await.keys().cloned().collect();

            for name in current.difference(&known) {
                log::debug!("adding profile '{name}'");
                match ProfileTokenManager::new_verified(name.clone(), Arc::clone(&cfg)).await {
                    Ok(m) => {
                        managers.write().await.insert(name.clone(), Arc::new(m));
                    }
                    Err(e) => {
                        log::warn!("failed to create manager for profile '{name}': {e:?}");
                    }
                }
            }

            for name in known.difference(&current) {
                log::debug!("removing profile '{name}'");
                if let Some(m) = managers.write().await.remove(name) {
                    m.stop();
                }
            }
        }
    }

    pub async fn for_profile(&self, name: &str) -> Option<Arc<ProfileTokenManager>> {
        self.managers.read().await.get(name).cloned()
    }
}

impl Drop for GlobalTokenManager {
    fn drop(&mut self) {
        GLOBAL_CREATED.store(false, Ordering::SeqCst);
    }
}
