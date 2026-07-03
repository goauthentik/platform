use crate::cache::{CacheData, ClientCache};
use eyre::{bail, Result, WrapErr};
use ak_platform::{
    client::user::{AnyService, Client},
    generated::{
        agent::RequestHeader,
        agent_auth::{CurrentTokenRequest, TokenExchangeRequest, current_token_request},
    },
    grpc::assert_response_valid,
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
#[serde(rename_all = "PascalCase")]
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

pub async fn get_credentials(
    c: Client<AnyService>,
    opts: CredentialsOpts,
) -> Result<AWSCredentialOutput> {
    let cc = ClientCache::new(
        c.clone(),
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

    let res = c
        .clone()
        .auth()
        .cached_token_exchange(TokenExchangeRequest {
            header: Some(RequestHeader {
                profile: opts.profile.clone(),
            }),
            client_id: opts.client_id.clone(),
        })
        .await
        .wrap_err("failed to exchange token")?
        .into_inner();
    assert_response_valid(res.header)?;

    let curr = c
        .clone()
        .auth()
        .get_current_token(CurrentTokenRequest {
            header: Some(RequestHeader {
                profile: opts.profile.clone(),
            }),
            r#type: current_token_request::Type::Verified as i32,
        })
        .await
        .wrap_err("failed to get current token")?
        .into_inner();
    assert_response_valid(curr.header)?;
    let username = curr
        .token
        .ok_or_else(|| eyre::eyre!("Failed to get current token"))?
        .preferred_username;

    tracing::debug!("Fetching AWS Credentials...");
    let aws_creds = sts
        .assume_role_with_web_identity()
        .role_arn(opts.role_arn)
        .role_session_name(username)
        .web_identity_token(res.access_token)
        .send()
        .await
        .wrap_err("failed to assume AWS role")?;

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
        cc.set(cached.clone()).await.wrap_err("failed to cache AWS credentials")?;
        return Ok(cached);
    }
    bail!("No credentials received")
}
