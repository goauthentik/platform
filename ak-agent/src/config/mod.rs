use std::collections::HashMap;

use reqwest::Client;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct ConfigV1 {
    pub debug: bool,
    pub profiles: HashMap<String, ConfigV1Profile>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ConfigV1Profile {
    pub authentik_url: String,
    pub app_slug: String,
    pub client_id: String,

    // Fallback if keyring isn't available
    #[serde(rename = "access_token")]
    pub fallback_access_token: String,
    #[serde(rename = "refresh_token")]
    pub fallback_refresh_token: String,

    // Not saved to JSON, loaded from keychain
    #[serde(skip)]
    pub _access_token: String,
    #[serde(skip)]
    pub _refresh_token: String,

    #[serde(skip)]
    _http_client: Option<Client>,
}

impl ConfigV1Profile {
    pub fn http_client(mut self) -> Client {
        match self._http_client {
            Some(c) => c,
            None => {
                let c = reqwest::Client::new();
                self._http_client = Some(c.clone());
                c
            }
        }
    }
}
