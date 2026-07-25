use ak_platform::storage::cfgmgr::schema::{Config as ConfigSchema, ConfigChangedType};
use eyre::Result;
use serde::{Deserialize, Serialize};

pub mod domain;
pub mod managed;

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct Config {
    pub debug: bool,
    pub runtime_dir: String,
    pub domain_dir: String,
}

impl ConfigSchema for Config {
    async fn post_load(&mut self) -> Result<()> {
        Ok(())
    }

    async fn pre_save(&mut self) -> Result<()> {
        Ok(())
    }

    async fn post_update(&self, _prev: Self) -> Result<ConfigChangedType> {
        Ok(ConfigChangedType::Generic)
    }
}
