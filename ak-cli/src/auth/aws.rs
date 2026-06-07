use std::error::Error;

use crate::cache::{CacheData, ClientCache};
use authentik_sys::{
    generated::{
        agent::RequestHeader,
        agent_auth::{
            CurrentTokenRequest, TokenExchangeRequest, agent_auth_client::AgentAuthClient,
            current_token_request,
        },
    },
    grpc::{assert_response_valid, grpc_endpoint},
    platform::paths::{AgentSocketID, agent_socket_path},
};
use aws_types::{SdkConfig, region::Region};
use pbjson_types::Timestamp;
use serde::{Deserialize, Serialize};

#[derive(Clone)]
pub struct CredentialsOpts {
    pub profile: String,
    pub client_id: String,
    // AWS specific things
    pub role_arn: String,
    pub region: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct AWSCredentialOutput {
    version: i32,
    access_key_id: String,
    secret_access_key: String,
    session_token: String,
    expiration: Timestamp,
}

impl CacheData for AWSCredentialOutput {
    fn expiry(&self) -> Timestamp {
        self.expiration
    }
}

pub async fn get_credentials(opts: CredentialsOpts) -> Result<AWSCredentialOutput, Box<dyn Error>> {
    let cc = ClientCache::new(
        RequestHeader {
            profile: opts.profile.clone(),
        },
        vec!["auth-aws-cache".to_string(), opts.role_arn.clone()],
    );
    if let Ok(v) = cc.get().await {
        return Ok(v);
    }

    let config = &SdkConfig::builder()
        .region(Region::new(opts.region.clone()))
        .build();
    let sts = aws_sdk_sts::Client::new(config);

    let c = grpc_endpoint(agent_socket_path(AgentSocketID::Default)?.for_current()).await?;
    let res = AgentAuthClient::new(c.clone())
        .cached_token_exchange(TokenExchangeRequest {
            header: Some(RequestHeader {
                profile: opts.profile.clone(),
            }),
            client_id: opts.client_id.clone(),
        })
        .await?
        .into_inner();
    assert_response_valid(res.header)?;

    let curr = AgentAuthClient::new(c.clone())
        .get_current_token(CurrentTokenRequest {
            header: Some(RequestHeader {
                profile: opts.profile.clone(),
            }),
            r#type: current_token_request::Type::Verified as i32,
        })
        .await?
        .into_inner();
    assert_response_valid(curr.header)?;
    let username = curr
        .token
        .ok_or("Failed to get current token")?
        .preferred_username;

    log::debug!("Fetching AWS Credentials...");
    let aws_creds = sts
        .assume_role_with_web_identity()
        .role_arn(opts.role_arn)
        .role_session_name(username)
        .web_identity_token(res.access_token)
        .send()
        .await?;

    if let Some(c) = aws_creds.credentials {
        let cached = AWSCredentialOutput {
            version: 1,
            access_key_id: c.access_key_id,
            secret_access_key: c.secret_access_key,
            session_token: c.session_token,
            expiration: Timestamp {
                seconds: c.expiration.secs(),
                nanos: c.expiration.subsec_nanos() as i32,
            },
        };
        cc.set(cached.clone()).await?;
        return Ok(cached);
    }
    Err(Box::from("No credentials received"))
}
