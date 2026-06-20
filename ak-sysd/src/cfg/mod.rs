use serde::{Deserialize, Serialize};
use ak_platform::storage::cfgmgr::schema::Config as ConfigSchema;

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Config {
    debug: bool,
    runtime_dir: String,
    domain_dir: String,
}

impl ConfigSchema for Config {

}
