use eyre::Result;
use serde::{Deserialize, Serialize};
use std::{fs, sync::LazyLock};

use crate::{
    paths::{SysdSocketID, sysd_socket_path},
    string::PlatformString,
};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Config {
    pub debug: bool,
    pub socket_default: PlatformString,
    pub socket_ctrl: PlatformString,
}

static GLOBAL_DATA: LazyLock<Config> = LazyLock::new(|| Config::from_default().unwrap_or_default());

impl Default for Config {
    fn default() -> Self {
        Config {
            debug: false,
            socket_default: sysd_socket_path(SysdSocketID::Default),
            socket_ctrl: sysd_socket_path(SysdSocketID::CTRL),
        }
    }
}

impl Config {
    pub fn get() -> Self {
        GLOBAL_DATA.clone()
    }

    pub fn from_default() -> Result<Self> {
        Config::from_file("/etc/authentik/config.json")
    }

    pub fn from_file(path: &str) -> Result<Self> {
        let content = fs::read_to_string(path)?;
        let config: Config = serde_json::from_str(&content)?;
        Ok(config)
    }
}
