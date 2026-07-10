use ak_platform::storage::cfgmgr::schema::{Config as ConfigSchema, ConfigChangedType};
use eyre::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct Config {
    debug: bool,
    runtime_dir: String,
    domain_dir: String,
}

impl ConfigSchema for Config {
    async fn post_load(&mut self) -> Result<()> {
        Ok(())
    }

    async fn pre_save(&self) -> Result<()> {
        Ok(())
    }

    async fn post_update(&self, _prev: Self) -> Result<ConfigChangedType> {
        Ok(ConfigChangedType::Generic)
    }
}
