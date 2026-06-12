use std::sync::Arc;
use std::time::Duration;

use jsonwebtoken::{DecodingKey, Validation, decode, decode_header, jwk::JwkSet};
use tokio::sync::{Notify, RwLock};

use ak_platform::prelude::*;
use ak_platform::storage::cfgmgr::ConfigManager;

use crate::config::ConfigV1;
use crate::token::{AuthentikClaims, Token};

pub struct ProfileTokenManager {
    profile_name: String,
    cfg: Arc<ConfigManager<ConfigV1>>,
    jwks: Option<Arc<RwLock<JwkSet>>>,
    cancel: Arc<Notify>,
}

impl ProfileTokenManager {
    pub async fn new_verified(
        profile_name: impl Into<String>,
        cfg: Arc<ConfigManager<ConfigV1>>,
    ) -> Result<Self> {
        let profile_name = profile_name.into();
        let jwks_url = {
            let config = cfg.read().await;
            let profile = config
                .profiles
                .get(&profile_name)
                .ok_or("profile not found")?;
            format!(
                "{}/application/o/{}/jwks/",
                profile.authentik_url, profile.app_slug
            )
        };

        let jwks = Self::fetch_jwks(&jwks_url).await?;
        let jwks = Arc::new(RwLock::new(jwks));
        let cancel = Arc::new(Notify::new());

        // start_renewing needs owned data since the task must be 'static
        let cancel_bg = Arc::clone(&cancel);
        let cfg_bg = Arc::clone(&cfg);
        let name_bg = profile_name.clone();
        tokio::spawn(async move {
            Self::start_renewing(name_bg, cfg_bg, cancel_bg).await;
        });

        Ok(ProfileTokenManager {
            profile_name,
            cfg,
            jwks: Some(jwks),
            cancel,
        })
    }

    pub fn new(profile_name: impl Into<String>, cfg: Arc<ConfigManager<ConfigV1>>) -> Self {
        ProfileTokenManager {
            profile_name: profile_name.into(),
            cfg,
            jwks: None,
            cancel: Arc::new(Notify::new()),
        }
    }

    pub async fn unverified(&self) -> Result<AuthentikClaims> {
        let raw = {
            let config = self.cfg.read().await;
            let profile = config
                .profiles
                .get(&self.profile_name)
                .ok_or("profile not found")?;
            profile._access_token.clone()
        };
        super::parse_unverified(&raw)
    }

    pub async fn token(&self) -> Result<Token> {
        let (raw, refresh) = {
            let config = self.cfg.read().await;
            let profile = config
                .profiles
                .get(&self.profile_name)
                .ok_or("profile not found")?;
            (
                profile._access_token.clone(),
                profile._refresh_token.clone(),
            )
        };

        if let Some(jwks) = &self.jwks {
            let guard = jwks.read().await;
            match Self::verify_token(&raw, &guard) {
                Ok(_) => {}
                Err(e) if Self::is_expired(&e) => {
                    log::debug!("token expired, renewing");
                    self.renew().await?;
                    let config = self.cfg.read().await;
                    let profile = config
                        .profiles
                        .get(&self.profile_name)
                        .ok_or("profile not found")?;
                    return Ok(Token {
                        access_token: profile._access_token.clone(),
                        token_type: None,
                        refresh_token: Some(profile._refresh_token.clone()),
                        expires_in: None,
                    });
                }
                Err(e) => {
                    log::warn!("token verification failed, returning unverified: {e:?}");
                }
            }
        }

        Ok(Token {
            access_token: raw,
            token_type: None,
            refresh_token: Some(refresh),
            expires_in: None,
        })
    }

    pub fn stop(&self) {
        self.cancel.notify_waiters();
    }

    // start_renewing takes owned args rather than &self because the spawned
    // task must be 'static — the same reason Go uses a goroutine with a pointer.
    async fn start_renewing(
        profile_name: String,
        cfg: Arc<ConfigManager<ConfigV1>>,
        cancel: Arc<Notify>,
    ) {
        loop {
            let sleep_dur = {
                let config = cfg.read().await;
                match config.profiles.get(&profile_name) {
                    None => {
                        log::warn!("profile '{profile_name}' not found, stopping renewal");
                        return;
                    }
                    Some(profile) => Self::time_until_expiry(&profile._access_token),
                }
            };

            log::debug!("profile '{profile_name}': renewing token in {sleep_dur:?}");

            tokio::select! {
                _ = tokio::time::sleep(sleep_dur) => {
                    log::debug!("profile '{profile_name}': renewing token now");
                    // Construct a temporary manager view to reuse renew()
                    let ptm = ProfileTokenManager {
                        profile_name: profile_name.clone(),
                        cfg: Arc::clone(&cfg),
                        jwks: None,
                        cancel: Arc::clone(&cancel),
                    };
                    if let Err(e) = ptm.renew().await {
                        log::warn!("profile '{profile_name}': failed to renew token: {e:?}");
                    }
                }
                _ = cancel.notified() => return,
            }
        }
    }

    async fn renew(&self) -> Result<()> {
        let (token_url, refresh_token, client_id) = {
            let config = self.cfg.read().await;
            let profile = config
                .profiles
                .get(&self.profile_name)
                .ok_or("profile not found")?;
            (
                format!("{}/application/o/token/", profile.authentik_url),
                profile._refresh_token.clone(),
                profile.client_id.clone(),
            )
        };

        let body = url::form_urlencoded::Serializer::new(String::new())
            .append_pair("grant_type", "refresh_token")
            .append_pair("refresh_token", &refresh_token)
            .finish();
        let client = reqwest::Client::new();
        let res = client
            .post(&token_url)
            .basic_auth(&client_id, None::<&str>)
            .header(
                reqwest::header::CONTENT_TYPE,
                "application/x-www-form-urlencoded",
            )
            .header(
                reqwest::header::USER_AGENT,
                format!("authentik-agent v{}", env!("CARGO_PKG_VERSION")),
            )
            .body(body)
            .send()
            .await?;

        if !res.status().is_success() {
            let body = res.text().await?;
            return Err(Box::from(format!("token renewal failed: {body}")));
        }

        let new_token: Token = res.json().await?;

        {
            let mut config = self.cfg.write().await;
            let profile = config
                .profiles
                .get_mut(&self.profile_name)
                .ok_or("profile not found")?;
            profile._access_token = new_token.access_token.clone();
            profile.fallback_access_token = new_token.access_token.clone();
            if let Some(rt) = &new_token.refresh_token
                && !rt.is_empty()
            {
                profile._refresh_token = rt.clone();
                profile.fallback_refresh_token = rt.clone();
            }
        }

        self.cfg.save().await?;
        log::debug!(
            "profile '{}': successfully refreshed token",
            self.profile_name
        );
        Ok(())
    }

    async fn fetch_jwks(url: &str) -> Result<JwkSet> {
        let res = reqwest::get(url).await?;
        let jwks = res.json::<JwkSet>().await?;
        Ok(jwks)
    }

    fn verify_token(
        token: &str,
        jwks: &JwkSet,
    ) -> std::result::Result<AuthentikClaims, jsonwebtoken::errors::Error> {
        let header = decode_header(token)?;
        let kid = header.kid.ok_or_else(|| {
            jsonwebtoken::errors::Error::from(jsonwebtoken::errors::ErrorKind::InvalidKeyFormat)
        })?;
        let jwk = jwks.find(&kid).ok_or_else(|| {
            jsonwebtoken::errors::Error::from(jsonwebtoken::errors::ErrorKind::InvalidKeyFormat)
        })?;
        let key = DecodingKey::from_jwk(jwk)?;
        let mut validation = Validation::new(header.alg);
        validation.validate_exp = true;
        let data = decode::<AuthentikClaims>(token, &key, &validation)?;
        Ok(data.claims)
    }

    fn time_until_expiry(token: &str) -> Duration {
        let Ok(claims) = super::parse_unverified(token) else {
            return Duration::from_secs(60);
        };
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        Duration::from_secs(claims.exp.saturating_sub(now))
    }

    fn is_expired(e: &jsonwebtoken::errors::Error) -> bool {
        matches!(e.kind(), jsonwebtoken::errors::ErrorKind::ExpiredSignature)
    }
}
