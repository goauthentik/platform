use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use tokio::sync::{Notify, RwLock};

use ak_platform::storage::cfgmgr::ConfigManager;
use eyre::{Result, bail};

use crate::config::ConfigV1;
use crate::token::profile::ProfileTokenManager;

static GLOBAL_CREATED: AtomicBool = AtomicBool::new(false);

pub struct GlobalTokenManager {
    cfg: Arc<ConfigManager<ConfigV1>>,
    managers: Arc<RwLock<HashMap<String, Arc<ProfileTokenManager>>>>,
    /// Profiles whose manager failed to be created (e.g. JWKS fetch failed
    /// because the authentik instance was unreachable), keyed by profile
    /// name with the error that caused creation to fail.
    failed: Arc<RwLock<HashMap<String, String>>>,
    reconcile_notify: Arc<Notify>,
}

impl GlobalTokenManager {
    pub async fn new(cfg: Arc<ConfigManager<ConfigV1>>) -> Result<Self> {
        if GLOBAL_CREATED.swap(true, Ordering::SeqCst) {
            bail!("only a single global token manager can be used");
        }
        let gtm = GlobalTokenManager {
            cfg: Arc::clone(&cfg),
            managers: Arc::new(RwLock::new(HashMap::new())),
            failed: Arc::new(RwLock::new(HashMap::new())),
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
                        self.failed.write().await.remove(name);
                    }
                    Err(e) => {
                        tracing::warn!(
                            profile = name,
                            "failed to create manager for profile: {e:?}"
                        );
                        self.failed
                            .write()
                            .await
                            .insert(name.clone(), e.to_string());
                    }
                }
            }
        }

        let cfg_bg = Arc::clone(&self.cfg);
        let managers_bg = Arc::clone(&self.managers);
        let failed_bg = Arc::clone(&self.failed);
        let reconcile_notify_bg = Arc::clone(&self.reconcile_notify);
        tokio::spawn(async move {
            Self::watch_config_changes(cfg_bg, managers_bg, failed_bg, reconcile_notify_bg).await;
        });
    }

    async fn watch_config_changes(
        cfg: Arc<ConfigManager<ConfigV1>>,
        managers: Arc<RwLock<HashMap<String, Arc<ProfileTokenManager>>>>,
        failed: Arc<RwLock<HashMap<String, String>>>,
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
                        failed.write().await.remove(name);
                    }
                    Err(e) => {
                        tracing::warn!(
                            profile = name,
                            "failed to create manager for profile: {e:?}"
                        );
                        failed.write().await.insert(name.clone(), e.to_string());
                    }
                }
            }

            for name in known.difference(&current) {
                tracing::debug!(profile = name, "removing profile");
                if let Some(m) = managers.write().await.remove(name) {
                    m.stop();
                }
                failed.write().await.remove(name);
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

    pub async fn for_profile<T: ToString>(&self, name: T) -> Option<Arc<ProfileTokenManager>> {
        self.managers.read().await.get(&name.to_string()).cloned()
    }

    /// Returns the error that caused manager creation to fail for a profile
    /// that has no live `ProfileTokenManager` (e.g. its JWKS fetch failed
    /// because the authentik instance was unreachable at startup).
    pub async fn creation_failure(&self, name: &str) -> Option<String> {
        self.failed.read().await.get(name).cloned()
    }
}

impl Drop for GlobalTokenManager {
    fn drop(&mut self) {
        GLOBAL_CREATED.store(false, Ordering::SeqCst);
    }
}

#[cfg(test)]
mod tests {
    use ak_platform::storage::cfgmgr::testutils::test_config_manager;

    use super::*;
    use crate::config::ConfigV1Profile;
    use crate::token::testutils::spawn_http_once;

    #[tokio::test]
    async fn one_profile_failing_does_not_block_others() {
        let _guard = crate::token::testutils::gtm_test_lock()
            .lock()
            .unwrap_or_else(|e| e.into_inner());

        let jwks_url = spawn_http_once("HTTP/1.1 200 OK", r#"{"keys":[]}"#).await;

        let cfg = test_config_manager::<ConfigV1>();
        {
            let mut c = cfg.write().await;
            c.profiles.insert(
                "good".to_string(),
                ConfigV1Profile::from_tokens(
                    jwks_url,
                    "app".to_string(),
                    "client".to_string(),
                    "access".to_string(),
                    "refresh".to_string(),
                ),
            );
            c.profiles.insert(
                "bad".to_string(),
                ConfigV1Profile::from_tokens(
                    // Nothing listens on port 1 on loopback: instant
                    // ConnectionRefused, simulating an unreachable instance.
                    "http://127.0.0.1:1".to_string(),
                    "app".to_string(),
                    "client".to_string(),
                    "access".to_string(),
                    "refresh".to_string(),
                ),
            );
        }

        let gtm = GlobalTokenManager::new(cfg).await.unwrap();

        assert!(gtm.for_profile("good").await.is_some());
        assert!(gtm.creation_failure("good").await.is_none());

        assert!(gtm.for_profile("bad").await.is_none());
        assert!(gtm.creation_failure("bad").await.is_some());
    }
}
