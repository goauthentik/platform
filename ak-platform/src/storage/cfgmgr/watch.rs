use std::{
    path::Path,
    sync::{Arc, mpsc},
};

use crate::prelude::*;
use crate::storage::cfgmgr::{ConfigManager, schema::Config};
use notify::{Event, EventKind, RecursiveMode, Watcher, recommended_watcher};

impl<T> ConfigManager<T>
where
    T: Config + 'static,
{
    pub fn watch(self: Arc<Self>) -> Result<()> {
        let (tx, rx) = mpsc::channel();
        let mut w = recommended_watcher(move |evr| {
            tx.send(evr).ok();
        })?;
        w.watch(
            Path::new(&self.path).parent().unwrap(),
            RecursiveMode::NonRecursive,
        )?;
        for evr in rx {
            let ev: Event = match evr {
                Ok(e) => e,
                Err(e) => {
                    log::warn!("error watching file: {e:?}");
                    continue;
                }
            };
            if let EventKind::Modify(_) = ev.kind {
                continue;
            }
            log::debug!("config file update");
            if let Err(e) = self.load() {
                log::warn!("failed to reload config: {e:?}");
            }
        }
        Ok(())
    }
}
