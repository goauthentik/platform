use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_with::formats::PreferOne;
use serde_with::{OneOrMany, serde_as};

pub const EXT_AUTHENTIK_PLATFORM_SSH_TOKEN: &str = "goauthentik.io/platform/ssh/ssh/token";
pub const EXT_AUTHENTIK_PLATFORM_SSH_HOST_KEY: &str = "goauthentik.io/platform/ssh/host-key";

#[serde_as]
#[derive(Debug, Serialize, Deserialize)]
pub struct AuthentikClaims {
    pub iss: String,
    // Handoff tokens (SSH-cert extension token, interactive-auth finish-redirect
    // token) are scoped by `preferred_username`, not a stable user `sub` — omit
    // rather than fail to decode when authentik doesn't emit it for these.
    #[serde(default)]
    pub sub: Option<String>,
    #[serde_as(as = "OneOrMany<_, PreferOne>")]
    pub aud: Vec<String>,
    #[serde(with = "chrono::serde::ts_seconds")]
    pub exp: DateTime<Utc>,
    #[serde(with = "chrono::serde::ts_seconds")]
    pub iat: DateTime<Utc>,
    pub jti: String,
    pub preferred_username: String,
}
