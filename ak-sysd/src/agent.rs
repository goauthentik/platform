#[cfg(target_os = "macos")]
use crate::components::{Component, ComponentConstructor, ping::PingComponent};
use crate::{cfg::Config, components::ComponentInstance};
use ak_platform::prelude::*;
use ak_platform::storage::cfgmgr::ConfigManager;
use std::{collections::HashMap, sync::Arc};

pub struct Agent {
    cfg: Arc<ConfigManager<Config>>,
    components: HashMap<String, ComponentInstance>,
}

impl Agent {
    pub async fn new(config_path: String) -> Result<Self> {
        let cfg = ConfigManager::new(config_path).await?;
        let comp = HashMap::new();

        let mut ag = Agent {
            cfg,
            components: comp,
        };
        ag.register_components().await?;
        Ok(ag)
    }

    pub async fn register_components(&mut self) -> Result<()> {
        for (name, constr) in Agent::register_platform_components() {
            let comp = (constr)()?;
            self.components.insert(name, ComponentInstance::new(comp));
        }
        Ok(())
    }

    pub async fn start() {}
}

#[cfg(target_os = "macos")]
impl Agent {
    pub fn register_platform_components() -> HashMap<String, ComponentConstructor> {
        HashMap::from([(PingComponent::id(), PingComponent::new as ComponentConstructor)])
    }
}
