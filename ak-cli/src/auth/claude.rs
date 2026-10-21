use ak_platform::{
    client::user::{AnyService, Client},
    generated::{agent::RequestHeader, agent_auth::TokenExchangeRequest},
    grpc::assert_response_valid,
};
use chrono::{TimeDelta, Utc};
use eyre::{Result, WrapErr};
use pbjson_types::Timestamp;
use serde::{Deserialize, Serialize};

use crate::cache::{CacheData, ClientCache};

pub struct CredentialsOpts {
    pub profile: String,
    pub client_id: String,
    pub federation_rule_id: String,
    pub organization_id: String,
    pub service_account_id: String,
    pub workspace_id: String,
}

#[derive(Serialize)]
struct ClaudeTokenExchange {
    federation_rule_id: String,
    organization_id: String,
    service_account_id: String,
    workspace_id: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ClaudeCredentialOutput {
    pub access_token: String,
    pub expiry: pbjson_types::Timestamp,
}

impl CacheData for ClaudeCredentialOutput {
    fn expiry(&self) -> pbjson_types::Timestamp {
        self.expiry
    }
}

pub async fn get_credentials(
    c: Client<AnyService>,
    opts: CredentialsOpts,
) -> Result<ClaudeCredentialOutput> {
    let cc = ClientCache::new(
        c.clone(),
        RequestHeader {
            profile: opts.profile.clone(),
        },
        vec![
            "auth-claude-cache".to_string(),
            opts.client_id.clone(),
            opts.federation_rule_id.clone(),
            opts.organization_id.clone(),
            opts.service_account_id.clone(),
            opts.workspace_id.clone(),
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
            actor_token: None,
            actor_token_type: None,
        })
        .await
        .wrap_err("failed to exchange token")?
        .into_inner();
    assert_response_valid(res.header)?;

    // Convert authentik token to Claude WIF token
    let client = reqwest::Client::builder()
        .user_agent(ak_meta::user_agent())
        .build()?;

    #[derive(Deserialize)]
    struct ClaudeTokenResponse {
        pub access_token: String,
        pub expires_in: i64,
    }

    let res: ClaudeTokenResponse = client
        .post("https://api.anthropic.com/v1/oauth/token")
        .json(&ClaudeTokenExchange {
            federation_rule_id: opts.federation_rule_id,
            organization_id: opts.organization_id,
            service_account_id: opts.service_account_id,
            workspace_id: opts.workspace_id,
        })
        .send()
        .await?
        // curl -sS happily returns error bodies, fail loudly instead
        .error_for_status()?
        .json()
        .await?;

    let cached = ClaudeCredentialOutput {
        access_token: res.access_token,
        expiry: Timestamp::from(Utc::now() + TimeDelta::seconds(res.expires_in)),
    };
    cc.set(cached.clone())
        .await
        .wrap_err("failed to cache Claude credentials")?;
    Ok(cached)
}
