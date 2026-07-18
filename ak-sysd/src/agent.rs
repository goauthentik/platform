use crate::components::{Component, ComponentConstructor, ping::PingComponent};
use crate::{cfg::Config, components::ComponentInstance};
use ak_platform::storage::cfgmgr::ConfigManager;
use eyre::Result;
use sentry_tower::{NewSentryLayer, SentryHttpLayer};
use std::{collections::HashMap, sync::Arc};
use tonic::transport::Server;
use tower_http::trace::{DefaultOnFailure, DefaultOnRequest, TraceLayer};
use tracing::Level;

pub struct Agent {
    cfg: Arc<ConfigManager<Config>>,
    components: HashMap<String, ComponentInstance>,
    srvs: Vec<Server>,
}

impl Agent {
    pub async fn new(config_path: String) -> Result<Self> {
        let cfg = ConfigManager::new(config_path).await?;
        let comp = HashMap::new();

        let t = Server::builder()
            .layer(NewSentryLayer::new_from_top())
            .layer(SentryHttpLayer::new().enable_transaction())
            .layer(
                TraceLayer::new_for_grpc()
                    .on_request(DefaultOnRequest::new().level(Level::INFO))
                    .on_failure(DefaultOnFailure::new().level(Level::ERROR)),
            );
        let mut ag = Agent {
            cfg,
            components: comp,
            srvs: vec![t],
        };
        ag.register_components().await?;
        Ok(ag)
    }

    pub async fn register_components(&mut self) -> Result<()> {
        for (name, constr) in Agent::register_platform_components() {
            let comp = (constr)()?;
            tracing::debug!(component = name, "Registering component");
            self.components.insert(name, ComponentInstance::new(comp));
        }
        Ok(())
    }

    pub async fn start(&self) -> Result<()> {
        Ok(())
    }
}

#[cfg(target_os = "macos")]
impl Agent {
    pub fn register_platform_components() -> HashMap<String, ComponentConstructor> {
        HashMap::from([(
            PingComponent::id(),
            PingComponent::new as ComponentConstructor,
        )])
    }
}
