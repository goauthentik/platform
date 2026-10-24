use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::{
    auth::k8s::{ExecCredential, ExecCredentialStatus},
    cache::{CacheData, ClientCache},
};
use ak_platform::{
    client::user::{AnyService, Client},
    generated::{
        agent::RequestHeader,
        agent_auth::{CurrentTokenRequest, TokenExchangeRequest, current_token_request},
    },
    grpc::assert_response_valid,
};
use aws_sdk_sts::config::Credentials;
use aws_sigv4::{
    http_request::{SignableBody, SignableRequest, SignatureLocation, SigningSettings, sign},
    sign::v4,
};
use aws_smithy_runtime_api::client::identity::Identity;
use aws_types::{SdkConfig, region::Region};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use eyre::{Result, WrapErr, bail};
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

#[derive(Clone)]
pub struct EKSCredentialOpts {
    pub opts: CredentialsOpts,
    pub cluster: String,
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
        opts.client_id.clone(),
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
            audience: opts.client_id.clone(),
            scopes: vec![],
            actor_token: None,
            actor_token_type: None,
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
            caller_pid: 0,
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
        cc.set(cached.clone())
            .await
            .wrap_err("failed to cache AWS credentials")?;
        return Ok(cached);
    }
    bail!("No credentials received")
}

pub async fn get_eks_credentials(
    c: Client<AnyService>,
    opts: EKSCredentialOpts,
) -> Result<ExecCredential> {
    let sts = get_credentials(c, opts.clone().opts).await?;

    let identity: Identity = Credentials::new(
        sts.access_key_id,
        sts.secret_access_key,
        Some(sts.session_token),
        Some(UNIX_EPOCH + Duration::from_secs(sts.expiration.seconds as u64)),
        "web-identity",
    )
    .into();

    // signature goes in the query string, same as the CLI
    let mut settings = SigningSettings::default();
    settings.signature_location = SignatureLocation::QueryParams;
    settings.expires_in = Some(Duration::from_secs(60));
    let region = opts.clone().opts.region;

    let params = v4::SigningParams::builder()
        .identity(&identity)
        .region(&region)
        .name("sts")
        .time(SystemTime::now())
        .settings(settings)
        .build()?
        .into();

    let url = format!(
        "https://sts.{}.amazonaws.com/?Action=GetCallerIdentity&Version=2011-06-15",
        region,
    );
    // EKS checks this header, so it must be signed
    let headers = [("x-k8s-aws-id", &*opts.cluster)];
    let signable =
        SignableRequest::new("GET", &url, headers.into_iter(), SignableBody::Bytes(&[]))?;
    let (instructions, _) = sign(signable, &params)?.into_parts();

    let mut req = http::Request::get(&url).body(())?;
    instructions.apply_to_request_http1x(&mut req);

    Ok(ExecCredential {
        api_version: Some("client.authentication.k8s.io/v1".to_string()),
        kind: Some("ExecCredential".to_string()),
        status: Some(ExecCredentialStatus {
            client_certificate_data: None,
            client_key_data: None,
            token: Some(format!(
                "k8s-aws-v1.{}",
                URL_SAFE_NO_PAD.encode(req.uri().to_string())
            )),
            expiration_timestamp: Some(sts.expiration.try_into().map_err(|e| eyre::eyre!("{e}"))?),
        }),
    })
}
