#[cfg(target_os = "macos")]
use crate::components::ComponentConstructor;
use crate::components::{Component, auth::AuthComponent, ping::{self, PingComponent}};
use crate::{cfg::Config, components::ComponentInstance};
use ak_platform::prelude::*;
use ak_platform::storage::cfgmgr::ConfigManager;
use std::{collections::HashMap, sync::Arc};
use crate::components::auth;

pub struct Agent {
    cfg: Arc<ConfigManager<Config>>,
    components: HashMap<String, ComponentInstance>,
}

impl Agent {
    pub async fn new(config_path: String) -> Result<Self> {
        let cfg = ConfigManager::new(config_path).await?;
        let comp = HashMap::new();

        Ok(Agent {
            cfg,
            components: comp,
        })
    }

    pub async fn start() {

    }
}

#[cfg(target_os = "macos")]
impl Agent {
    pub fn register_platform_components() -> HashMap<String, ComponentConstructor> {

        return HashMap::from([
            (PingComponent::id(), PingComponent::new),
            // (auth::ID, AuthComponent),
        ])
    }
}
