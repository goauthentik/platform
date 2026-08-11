use std::{path::Path, sync::Arc};

use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use crate::storage::cfgmgr::{ConfigManager, schema::Config};
use eyre::Result;
use notify::{Event, EventKind, RecursiveMode, Watcher, recommended_watcher};

impl<T> ConfigManager<T>
where
    T: Config + 'static,
{
    pub async fn watch(self: Arc<Self>, cancel: CancellationToken) -> Result<()> {
        let (tx, mut rx) = mpsc::channel(64);
        let mut w = recommended_watcher(move |evr| {
            tx.blocking_send(evr).ok();
        })?;
        w.watch(
            Path::new(&self.path).parent().unwrap(),
            RecursiveMode::NonRecursive,
        )?;

        loop {
            tokio::select! {
                () = cancel.cancelled() => break,
                evr = rx.recv() => {
                    let Some(evr) = evr else { break };
                    let ev: Event = match evr {
                        Ok(e) => e,
                        Err(e) => {
                            tracing::warn!("error watching file: {e:?}");
                            continue;
                        }
                    };
                    if let EventKind::Access(_) = ev.kind {
                        continue;
                    }
                    tracing::debug!("config file update");
                    if let Err(e) = self.load().await {
                        tracing::warn!("failed to reload config: {e:?}");
                    }
                }
            }
        }

        // Some platforms' `Watcher` impls (e.g. FSEvents on macOS) block on a
        // native thread join when dropped; do that off the async executor so
        // a wedged native thread can't stall a tokio worker.
        tokio::task::spawn_blocking(move || drop(w)).await.ok();
        Ok(())
    }
}
