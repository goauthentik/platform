#[cfg(unix)]
use std::os::unix::fs::OpenOptionsExt;
use std::{
    fs::{File, OpenOptions, create_dir_all},
    io::ErrorKind,
    marker::PhantomData,
    path::Path,
    sync::Arc,
};
use tokio::sync::{Notify, RwLock, RwLockReadGuard, RwLockWriteGuard};

use crate::{prelude::*, storage::cfgmgr::schema::Config};

pub mod schema;
pub mod watch;

#[derive(Debug)]
pub struct ConfigManager<T: Config> {
    path: String,
    loaded: RwLock<T>,
    reload_notify: Arc<Notify>,

    _phantom: PhantomData<T>,
}

impl<T> ConfigManager<T>
where
    T: Config + 'static,
{
    pub async fn new(path: String) -> Result<Arc<Self>> {
        let cm = ConfigManager {
            path,
            loaded: RwLock::new(T::default()),
            reload_notify: Arc::new(Notify::new()),
            _phantom: PhantomData,
        };
        tracing::debug!("Config file path: {}", cm.path);
        if let Some(parent) = Path::new(&cm.path).parent() {
            tracing::debug!("Creating parent config dir: {}", parent.to_string_lossy());
            create_dir_all(parent)?;
        }
        cm.load().await?;
        tracing::debug!("Starting config watch");
        let shared = Arc::new(cm);
        let watch_arc = Arc::clone(&shared);
        let res_arc = Arc::clone(&watch_arc);
        tokio::spawn(async move {
            match watch_arc.watch().await {
                Ok(_) => (),
                Err(e) => {
                    tracing::warn!("failed to watch files: {e:?}");
                }
            };
        });
        Ok(res_arc)
    }

    pub fn get(self) -> T {
        self.loaded.into_inner()
    }

    pub fn on_reload(&self) -> Arc<Notify> {
        Arc::clone(&self.reload_notify)
    }

    pub fn notify_reload(&self) {
        self.reload_notify.notify_waiters();
    }

    pub async fn read(&self) -> RwLockReadGuard<'_, T> {
        self.loaded.read().await
    }

    pub async fn write(&self) -> RwLockWriteGuard<'_, T> {
        self.loaded.write().await
    }

    #[tracing::instrument]
    pub async fn load(&self) -> Result<()> {
        tracing::debug!("Loading config");
        let file = match File::open(self.path.clone()) {
            Ok(f) => f,
            Err(e) => match e.kind() {
                ErrorKind::NotFound => {
                    tracing::debug!("File not found, loading defaults");
                    *self.loaded.write().await = T::default();
                    return Ok(());
                }
                _ => {
                    return Err(e.into());
                }
            },
        };

        let mut new_val: T = serde_json::from_reader(file)?;
        new_val.post_load().await?;
        *self.loaded.write().await = new_val;
        self.reload_notify.notify_waiters();
        Ok(())
    }

    #[tracing::instrument]
    pub async fn save(&self) -> Result<()> {
        let loaded = self.loaded.read().await;
        loaded.pre_save().await?;
        tracing::debug!("saving config");
        let mut opts = OpenOptions::new();
        opts.create(true).truncate(true).read(true).write(true);
        #[cfg(unix)]
        {
            opts.mode(0o600);
        }
        let file = opts.open(self.path.clone())?;
        serde_json::to_writer(file, &*loaded)?;
        self.notify_reload();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    };
    use std::time::Duration;

    use serde::{Deserialize, Serialize};
    use tempfile::TempDir;

    use super::*;
    use crate::storage::cfgmgr::schema::Config;

    #[derive(Serialize, Deserialize, Debug)]
    struct TestCfg {
        field: String,
        #[serde(skip)]
        post_load_called: Arc<AtomicBool>,
        #[serde(skip)]
        pre_save_called: Arc<AtomicBool>,
    }

    impl Default for TestCfg {
        fn default() -> Self {
            TestCfg {
                field: String::new(),
                post_load_called: Arc::new(AtomicBool::new(false)),
                pre_save_called: Arc::new(AtomicBool::new(false)),
            }
        }
    }

    impl Config for TestCfg {
        async fn post_load(&mut self) -> crate::prelude::Result<()> {
            self.post_load_called.store(true, Ordering::SeqCst);
            Ok(())
        }
        async fn pre_save(&self) -> crate::prelude::Result<()> {
            self.pre_save_called.store(true, Ordering::SeqCst);
            Ok(())
        }
    }

    fn temp_file(dir: &TempDir, content: &str) -> String {
        let path = dir.path().join("config.json");
        std::fs::write(&path, content).unwrap();
        path.to_str().unwrap().to_string()
    }

    #[tokio::test]
    async fn test_load() {
        let dir = TempDir::new().unwrap();
        let path = temp_file(&dir, r#"{"field":"foo"}"#);

        let mgr = ConfigManager::<TestCfg>::new(path).await.unwrap();
        assert_eq!(mgr.loaded.read().await.field, "foo");

        mgr.loaded.write().await.field = "fo".into();
        mgr.save().await.unwrap();
    }

    #[tokio::test]
    async fn test_hooks() {
        let dir = TempDir::new().unwrap();
        let path = temp_file(&dir, r#"{"field":"foo"}"#);

        let mgr = ConfigManager::<TestCfg>::new(path).await.unwrap();
        let post_load = Arc::clone(&mgr.loaded.read().await.post_load_called);
        let pre_save = Arc::clone(&mgr.loaded.read().await.pre_save_called);

        mgr.save().await.unwrap();
        assert!(pre_save.load(Ordering::SeqCst));

        mgr.load().await.unwrap();
        assert!(post_load.load(Ordering::SeqCst));
    }

    #[tokio::test]
    async fn test_load_invalid() {
        let dir = TempDir::new().unwrap();
        let path = temp_file(&dir, r#"{"field":"foo}"#);

        assert!(ConfigManager::<TestCfg>::new(path).await.is_err());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_reload() {
        let dir = TempDir::new().unwrap();
        let path = temp_file(&dir, r#"{"field":"foo"}"#);

        let mgr = ConfigManager::<TestCfg>::new(path.clone()).await.unwrap();
        assert_eq!(mgr.loaded.read().await.field, "foo");

        // Allow the watcher thread to start and register with the OS.
        std::thread::sleep(Duration::from_millis(500));

        // Write new content to config file (Modify event — watcher skips this),
        // then create an unrelated file in the same dir (Create event — watcher
        // fires, reloads config.json which now has "bar").
        std::fs::write(&path, r#"{"field":"bar"}"#).unwrap();
        std::fs::write(dir.path().join("trigger"), "").unwrap();

        std::thread::sleep(Duration::from_secs(5));
        assert_eq!(mgr.loaded.read().await.field, "bar");
    }
}
