use std::{path::Path, sync::Arc};

use tokio::sync::mpsc;

use crate::prelude::*;
use crate::storage::cfgmgr::{ConfigManager, schema::Config};
use notify::{Event, EventKind, RecursiveMode, Watcher, recommended_watcher};

impl<T> ConfigManager<T>
where
    T: Config + 'static,
{
    pub async fn watch(self: Arc<Self>) -> Result<()> {
        let (tx, mut rx) = mpsc::channel(64);
        let mut w = recommended_watcher(move |evr| {
            tx.blocking_send(evr).ok();
        })?;
        w.watch(
            Path::new(&self.path).parent().unwrap(),
            RecursiveMode::NonRecursive,
        )?;
        while let Some(evr) = rx.recv().await {
            let ev: Event = match evr {
                Ok(e) => e,
                Err(e) => {
                    log::warn!("error watching file: {e:?}");
                    continue;
                }
            };
            if let EventKind::Access(_) = ev.kind { continue }
            log::debug!("config file update");
            if let Err(e) = self.load().await {
                log::warn!("failed to reload config: {e:?}");
            }
        }
        Ok(())
    }
}
