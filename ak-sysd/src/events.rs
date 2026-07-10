use tokio::sync::broadcast;

/// Replaces Go's `*events.Bus` (stringly-typed topic/payload pub-sub) with a
/// closed enum — a deliberate improvement, not a straight port.
#[derive(Clone, Debug)]
pub enum SysdEvent {
    LifecycleStarted,
    DirectoryFetched { domain: String },
    SessionOpened { session_id: String, pid: u32 },
    ConfigChanged { kind: ConfigChangeKind },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConfigChangeKind {
    Generic,
    Added,
    Removed,
}

#[derive(Clone)]
pub struct EventBus {
    tx: broadcast::Sender<SysdEvent>,
}

impl EventBus {
    pub fn new() -> Self {
        let (tx, _rx) = broadcast::channel(128);
        Self { tx }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<SysdEvent> {
        self.tx.subscribe()
    }

    pub fn dispatch(&self, ev: SysdEvent) {
        // send() only errors when there are no receivers — nothing to react
        // to yet is a normal state for a daemon at startup, not a failure.
        if self.tx.send(ev.clone()).is_err() {
            tracing::debug!(?ev, "dispatched event with no active subscribers");
        }
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}
