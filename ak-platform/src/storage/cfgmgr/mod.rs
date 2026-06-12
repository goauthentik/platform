use std::{
    fs::{File, OpenOptions},
    io::ErrorKind,
    marker::PhantomData,
    sync::{Arc, RwLock},
};

use crate::{prelude::*, storage::cfgmgr::schema::Config};

pub mod schema;
pub mod watch;

pub struct ConfigManager<T: Config> {
    path: String,
    loaded: RwLock<T>,

    _phantom: PhantomData<T>,
}

impl<T> ConfigManager<T>
where
    T: Config + 'static,
{
    pub fn new(path: String) -> Result<Arc<Self>> {
        let cm = ConfigManager {
            path,
            loaded: RwLock::new(T::default()),
            _phantom: PhantomData,
        };
        log::debug!("Config file path: {}", cm.path);
        cm.load()?;
        log::debug!("Starting config watch");
        let shared = Arc::new(cm);
        let watch_arc = Arc::clone(&shared);
        let res_arc = Arc::clone(&watch_arc);
        std::thread::spawn(move || {
            match watch_arc.watch() {
                Ok(_) => (),
                Err(e) => {
                    log::warn!("failed to watch files: {e:?}");
                }
            };
        });
        Ok(res_arc)
    }

    pub fn get(self) -> T {
        self.loaded.into_inner().unwrap()
    }

    pub fn load(&self) -> Result<()> {
        log::debug!("Loading config");
        let file = match File::open(self.path.clone()) {
            Ok(f) => f,
            Err(e) => match e.kind() {
                ErrorKind::NotFound => {
                    log::debug!("File not found, loading defaults");
                    *self.loaded.write().unwrap() = T::default();
                    return Ok(());
                }
                _ => {
                    return Err(e.into());
                }
            },
        };

        let new_val: T = serde_json::from_reader(file)?;
        new_val.post_load()?;
        *self.loaded.write().unwrap() = new_val;
        Ok(())
    }

    pub fn save(&self) -> Result<()> {
        let loaded = self.loaded.read().unwrap();
        loaded.pre_save()?;
        log::debug!("saving config");
        let file = OpenOptions::new()
            .create(true)
            .truncate(true)
            .read(true)
            .write(true)
            .open(self.path.clone())?;
        serde_json::to_writer(file, &*loaded)?;
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

    #[derive(Serialize, Deserialize)]
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
        fn post_load(&self) -> crate::prelude::Result<()> {
            self.post_load_called.store(true, Ordering::SeqCst);
            Ok(())
        }
        fn pre_save(&self) -> crate::prelude::Result<()> {
            self.pre_save_called.store(true, Ordering::SeqCst);
            Ok(())
        }
    }

    fn temp_file(dir: &TempDir, content: &str) -> String {
        let path = dir.path().join("config.json");
        std::fs::write(&path, content).unwrap();
        path.to_str().unwrap().to_string()
    }

    #[test]
    fn test_load() {
        let dir = TempDir::new().unwrap();
        let path = temp_file(&dir, r#"{"field":"foo"}"#);

        let mgr = ConfigManager::<TestCfg>::new(path).unwrap();
        assert_eq!(mgr.loaded.read().unwrap().field, "foo");

        mgr.loaded.write().unwrap().field = "fo".into();
        mgr.save().unwrap();
    }

    #[test]
    fn test_hooks() {
        let dir = TempDir::new().unwrap();
        let path = temp_file(&dir, r#"{"field":"foo"}"#);

        let mgr = ConfigManager::<TestCfg>::new(path).unwrap();
        let post_load = Arc::clone(&mgr.loaded.read().unwrap().post_load_called);
        let pre_save = Arc::clone(&mgr.loaded.read().unwrap().pre_save_called);

        mgr.save().unwrap();
        assert!(pre_save.load(Ordering::SeqCst));

        mgr.load().unwrap();
        assert!(post_load.load(Ordering::SeqCst));
    }

    #[test]
    fn test_load_invalid() {
        let dir = TempDir::new().unwrap();
        let path = temp_file(&dir, r#"{"field":"foo}"#);

        assert!(ConfigManager::<TestCfg>::new(path).is_err());
    }

    #[test]
    fn test_reload() {
        let dir = TempDir::new().unwrap();
        let path = temp_file(&dir, r#"{"field":"foo"}"#);

        let mgr = ConfigManager::<TestCfg>::new(path.clone()).unwrap();
        assert_eq!(mgr.loaded.read().unwrap().field, "foo");

        // Allow the watcher thread to start and register with the OS.
        std::thread::sleep(Duration::from_millis(500));

        // Write new content to config file (Modify event — watcher skips this),
        // then create an unrelated file in the same dir (Create event — watcher
        // fires, reloads config.json which now has "bar").
        std::fs::write(&path, r#"{"field":"bar"}"#).unwrap();
        std::fs::write(dir.path().join("trigger"), "").unwrap();

        std::thread::sleep(Duration::from_secs(5));
        assert_eq!(mgr.loaded.read().unwrap().field, "bar");
    }
}
