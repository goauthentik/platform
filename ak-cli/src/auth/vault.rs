use ak_platform::{
    client::user::{AnyService, Client},
    generated::{agent::RequestHeader, agent_auth::TokenExchangeRequest},
    grpc::assert_response_valid,
    http::ReqwestResponseError,
};
use chrono::{TimeDelta, Utc};
use eyre::{Result, WrapErr};
use pbjson_types::Timestamp;
use serde::{Deserialize, Serialize};

use crate::cache::{CacheData, ClientCache};

pub struct CredentialsOpts {
    pub profile: String,
    pub client_id: String,
    pub address: String,
    pub mount: String,
    pub role: String,
}

#[derive(Serialize)]
struct VaultLogin {
    role: String,
    jwt: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct VaultCredentialOutput {
    pub client_token: String,
    pub expiry: Timestamp,
}

impl CacheData for VaultCredentialOutput {
    fn expiry(&self) -> Timestamp {
        self.expiry
    }
}

pub async fn get_credentials(
    c: Client<AnyService>,
    opts: CredentialsOpts,
) -> Result<VaultCredentialOutput> {
    let cc = ClientCache::new(
        c.clone(),
        RequestHeader {
            profile: opts.profile.clone(),
        },
        opts.client_id.clone(),
        vec![
            "auth-vault-cache".to_string(),
            opts.address.clone(),
            opts.mount.clone(),
            opts.role.clone(),
        ],
    );
    if let Ok(v) = cc.get().await {
        return Ok(v);
    }

    let res = c
        .auth()
        .cached_token_exchange(TokenExchangeRequest {
            header: Some(RequestHeader {
                profile: opts.profile,
            }),
            audience: opts.client_id,
            scopes: vec![],
            ..Default::default()
        })
        .await
        .wrap_err("failed to exchange token")?
        .into_inner();
    assert_response_valid(res.header)?;

    let client = reqwest::Client::builder()
        .user_agent(ak_meta::user_agent())
        .build()?;

    #[derive(Deserialize)]
    struct VaultAuth {
        client_token: String,
        lease_duration: i64,
    }
    #[derive(Deserialize)]
    struct VaultLoginResponse {
        auth: VaultAuth,
    }

    // Must be a JWT-mode mount: Vault rejects JWT logins on mounts configured with an
    // OIDC client ID/secret ("unsupported config type").
    let res: VaultLoginResponse = client
        .post(format!(
            "{}/v1/auth/{}/login",
            opts.address.trim_end_matches('/'),
            opts.mount.trim_matches('/'),
        ))
        .json(&VaultLogin {
            role: opts.role,
            jwt: res.access_token,
        })
        .send()
        .await?
        .error_with_response()
        .await?
        .json()
        .await?;

    let cached = VaultCredentialOutput {
        client_token: res.auth.client_token,
        expiry: Timestamp::from(Utc::now() + TimeDelta::seconds(res.auth.lease_duration)),
    };
    cc.set(cached.clone())
        .await
        .wrap_err("failed to cache Vault credentials")?;
    Ok(cached)
}
