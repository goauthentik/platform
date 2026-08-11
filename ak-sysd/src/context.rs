use eyre::Result;
use std::any::Any;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use tokio_util::sync::CancellationToken;

use crate::cfg::Config;
use crate::cfg::domain::DomainManager;
use crate::events::EventBus;
use crate::state::StateStore;
use ak_platform::storage::cfgmgr::ConfigManager;

/// Maps component id -> concrete component instance, downcast on lookup.
/// Mirrors Go's `component.Get[T]` helper (`pkg/agent_system/component/component.go`).
///
/// Backed by a `std::sync::RwLock`, not a tokio lock: callers (`ctrl`,
/// `auth::token`) look components up synchronously with no `.await`.
#[derive(Clone, Default, Debug)]
pub struct ComponentRegistry {
    inner: Arc<RwLock<HashMap<&'static str, Arc<dyn Any + Send + Sync>>>>,
}

impl ComponentRegistry {
    pub fn insert<T: Any + Send + Sync + 'static>(&self, id: &'static str, comp: Arc<T>) {
        let mut map = self.inner.write().unwrap_or_else(|e| e.into_inner());
        map.insert(id, comp as Arc<dyn Any + Send + Sync>);
    }

    pub fn get<T: Any + Send + Sync + 'static>(&self, id: &str) -> Option<Arc<T>> {
        let map = self.inner.read().unwrap_or_else(|e| e.into_inner());
        map.get(id)?.clone().downcast::<T>().ok()
    }
}

/// Shared handles every component needs, mirroring Go's `component.Context`.
/// Cheap to `Clone` — every field is an `Arc`/handle type.
#[derive(Clone, Debug)]
pub struct SysdContext {
    pub cfg: Arc<ConfigManager<Config>>,
    pub domains: Arc<DomainManager>,
    pub state: Arc<StateStore>,
    pub events: EventBus,
    pub cancel: CancellationToken,
    pub registry: ComponentRegistry,
}

impl SysdContext {
    pub fn new(
        cfg: Arc<ConfigManager<Config>>,
        domains: Arc<DomainManager>,
        state: Arc<StateStore>,
        cancel: CancellationToken,
    ) -> Result<Self> {
        Ok(Self {
            cfg,
            domains,
            state,
            events: EventBus::new(),
            cancel,
            registry: ComponentRegistry::default(),
        })
    }
}

#[cfg(any(test, debug_assertions))]
#[allow(clippy::unwrap_used)]
pub mod testutils {
    use std::sync::Arc;

    use ak_platform::storage::cfgmgr::{ConfigManager, testutils::test_config_manager};
    use tokio_util::sync::CancellationToken;

    use crate::{
        cfg::{Config, domain::testutils::test_manager},
        context::SysdContext,
        state::testutils::test_store,
    };

    pub async fn test_context() -> SysdContext {
        let cm: Arc<ConfigManager<Config>> = test_config_manager();
        let state = Arc::new(test_store());
        let domains = test_manager(Arc::clone(&state)).await;
        SysdContext::new(cm, domains, state, CancellationToken::new()).unwrap()
    }
}
