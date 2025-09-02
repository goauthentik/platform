use serde::{Deserialize, Serialize};
use std::{error::Error, fs, sync::LazyLock};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PAMConfig {
    pub authentication_flow: String,
    pub terminate_on_expiry: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Config {
    pub authentik_url: String,
    pub app_slug: String,
    pub debug: bool,
    pub socket: String,
    pub pam: PAMConfig,
}

static GLOBAL_DATA: LazyLock<Config> =
    LazyLock::new(|| Config::from_default().unwrap_or(Config::default()));

impl Config {
    pub fn default() -> Self {
        Config {
            authentik_url: "".to_string(),
            app_slug: "".to_string(),
            debug: false,
            socket: "/var/run/authentik/sys.sock".to_string(),
            pam: PAMConfig {
                authentication_flow: "".to_string(),
                terminate_on_expiry: true,
            },
        }
    }

    pub fn get() -> Self {
        GLOBAL_DATA.clone()
    }

    pub fn from_default() -> Result<Self, Box<dyn Error>> {
        Config::from_file("/etc/authentik/host.yaml")
    }

    pub fn from_file(path: &str) -> Result<Self, Box<dyn Error>> {
        let content = fs::read_to_string(path)?;
        let config: Config = serde_yaml::from_str(&content)?;
        Ok(config)
    }
}
