use serde::{Deserialize, Serialize};
use std::{error::Error, fs, sync::LazyLock};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Config {
    pub debug: bool,
    pub socket: String,
}

static GLOBAL_DATA: LazyLock<Config> = LazyLock::new(|| Config::from_default().unwrap_or_default());

impl Default for Config {
    fn default() -> Self {
        Config {
            debug: false,
            #[cfg(unix)]
            socket: "/var/run/authentik/sys.sock".to_string(),
            #[cfg(windows)]
            socket: r"\\.\pipe\authentik\sysd".to_string(),
        }
    }
}

impl Config {
    pub fn get() -> Self {
        GLOBAL_DATA.clone()
    }

    pub fn from_default() -> Result<Self, Box<dyn Error>> {
        Config::from_file("/etc/authentik/config.json")
    }

    pub fn from_file(path: &str) -> Result<Self, Box<dyn Error>> {
        let content = fs::read_to_string(path)?;
        let config: Config = serde_json::from_str(&content)?;
        Ok(config)
    }
}
