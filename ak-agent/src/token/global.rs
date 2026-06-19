use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use tokio::sync::{Notify, RwLock};

use ak_platform::prelude::*;
use ak_platform::storage::cfgmgr::ConfigManager;

use crate::config::ConfigV1;
use crate::token::profile::ProfileTokenManager;

static GLOBAL_CREATED: AtomicBool = AtomicBool::new(false);

pub struct GlobalTokenManager {
    cfg: Arc<ConfigManager<ConfigV1>>,
    managers: Arc<RwLock<HashMap<String, Arc<ProfileTokenManager>>>>,
    reconcile_notify: Arc<Notify>,
}

impl GlobalTokenManager {
    pub async fn new(cfg: Arc<ConfigManager<ConfigV1>>) -> Result<Self> {
        if GLOBAL_CREATED.swap(true, Ordering::SeqCst) {
            return Err(Box::from("only a single global token manager can be used"));
        }
        let gtm = GlobalTokenManager {
            cfg: Arc::clone(&cfg),
            managers: Arc::new(RwLock::new(HashMap::new())),
            reconcile_notify: Arc::new(Notify::new()),
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
                        tracing::warn!(profile = name, "failed to create manager for profile: {e:?}");
                    }
                }
            }
        }

        let cfg_bg = Arc::clone(&self.cfg);
        let managers_bg = Arc::clone(&self.managers);
        let reconcile_notify_bg = Arc::clone(&self.reconcile_notify);
        tokio::spawn(async move {
            Self::watch_config_changes(cfg_bg, managers_bg, reconcile_notify_bg).await;
        });
    }

    async fn watch_config_changes(
        cfg: Arc<ConfigManager<ConfigV1>>,
        managers: Arc<RwLock<HashMap<String, Arc<ProfileTokenManager>>>>,
        reconcile_notify: Arc<Notify>,
    ) {
        let notify = cfg.on_reload();
        loop {
            notify.notified().await;

            let current: HashSet<String> = cfg.read().await.profiles.keys().cloned().collect();
            let known: HashSet<String> = managers.read().await.keys().cloned().collect();

            for name in current.difference(&known) {
                tracing::debug!(profile = name, "adding profile");
                match ProfileTokenManager::new_verified(name.clone(), Arc::clone(&cfg)).await {
                    Ok(m) => {
                        managers.write().await.insert(name.clone(), Arc::new(m));
                    }
                    Err(e) => {
                        tracing::warn!(profile = name, "failed to create manager for profile: {e:?}");
                    }
                }
            }

            for name in known.difference(&current) {
                tracing::debug!(profile = name,"removing profile");
                if let Some(m) = managers.write().await.remove(name) {
                    m.stop();
                }
            }

            reconcile_notify.notify_waiters();
        }
    }

    /// Waits until the named profile has been reconciled into the manager map.
    /// Must be called after the profile has been written to cfg.
    pub async fn wait_for_profile(&self, name: &str) {
        loop {
            // Register before checking to avoid missing a notification that
            // fires between the check and the await.
            let notified = self.reconcile_notify.notified();
            tokio::pin!(notified);
            notified.as_mut().enable();

            if self.managers.read().await.contains_key(name) {
                return;
            }
            notified.await;
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
