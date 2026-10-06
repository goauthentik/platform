use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::fmt;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, RwLock};

/// Anything dispatchable on the [`EventBus`]. Topics are plain types declared
/// next to the component that owns them — the bus keys on `TypeId`, so
/// there's no central enum to extend when a component grows an event.
pub trait Event: Any + Clone + fmt::Debug + Send + Sync + 'static {}
impl<T: Any + Clone + fmt::Debug + Send + Sync + 'static> Event for T {}

type Handler =
    Arc<dyn Fn(&dyn Any) -> Pin<Box<dyn Future<Output = ()> + Send>> + Send + Sync + 'static>;

#[derive(Clone, Default)]
pub struct EventBus {
    handlers: Arc<RwLock<HashMap<TypeId, Vec<Handler>>>>,
}

impl EventBus {
    pub fn new() -> Self {
        Self::default()
    }

    /// Run `f` on every dispatch of topic `E`. Register at construction time,
    /// not in `Component::start`, or a restart doubles up the handlers.
    pub fn on<E, F, Fut>(&self, f: F)
    where
        E: Event,
        F: Fn(E) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        let handler: Handler = Arc::new(
            move |ev: &dyn Any| -> Pin<Box<dyn Future<Output = ()> + Send>> {
                // dispatch only ever calls handlers filed under E's own TypeId.
                match ev.downcast_ref::<E>() {
                    Some(ev) => Box::pin(f(ev.clone())),
                    None => Box::pin(std::future::ready(())),
                }
            },
        );
        self.handlers
            .write()
            .unwrap_or_else(|e| e.into_inner())
            .entry(TypeId::of::<E>())
            .or_default()
            .push(handler);
    }

    /// Fire-and-forget: each handler gets its own task, so a slow one can't
    /// block the dispatcher or its siblings.
    pub fn dispatch<E: Event>(&self, ev: E) {
        let handlers = self
            .handlers
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .get(&TypeId::of::<E>())
            .cloned()
            .unwrap_or_default();
        tracing::debug!(
            topic = std::any::type_name::<E>(),
            handlers = handlers.len(),
            ?ev,
            "dispatching event"
        );
        for h in handlers {
            tokio::spawn(h(&ev));
        }
    }
}

impl fmt::Debug for EventBus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let n = self
            .handlers
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .values()
            .map(Vec::len)
            .sum::<usize>();
        f.debug_struct("EventBus").field("handlers", &n).finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::mpsc;

    #[derive(Clone, Debug)]
    struct Ping(u32);
    #[derive(Clone, Debug)]
    struct Pong;

    #[tokio::test]
    async fn handlers_fire_only_for_their_own_topic() {
        let bus = EventBus::new();
        let (tx, mut rx) = mpsc::unbounded_channel();
        let tx2 = tx.clone();
        bus.on(move |p: Ping| {
            let tx = tx.clone();
            async move {
                let _ = tx.send(p.0);
            }
        });
        bus.on(move |_: Pong| {
            let tx = tx2.clone();
            async move {
                let _ = tx.send(999);
            }
        });

        bus.dispatch(Ping(7));
        assert_eq!(rx.recv().await, Some(7));
    }
}
