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
