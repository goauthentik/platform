use eyre::{Result, WrapErr};
use ak_platform::{
    client::user::{AnyService, Client},
    generated::{agent::RequestHeader, agent_auth::TokenExchangeRequest},
    grpc::assert_response_valid,
};
use chrono::{DateTime, TimeDelta, Utc};
use serde::{Deserialize, Serialize};

#[derive(Clone)]
pub struct CredentialsOpts {
    pub profile: String,
    pub client_id: String,
}

// Models extracted from kube-rs as they are private in that crate and we only need
// the data structures

/// ExecCredentials is used by exec-based plugins to communicate credentials to
/// HTTP transports.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExecCredential {
    pub kind: Option<String>,
    #[serde(rename = "apiVersion")]
    pub api_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<ExecCredentialStatus>,
}

/// ExecCredentialStatus holds credentials for the transport to use.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExecCredentialStatus {
    #[serde(rename = "expirationTimestamp")]
    pub expiration_timestamp: Option<DateTime<Utc>>,
    pub token: Option<String>,
    #[serde(rename = "clientCertificateData")]
    pub client_certificate_data: Option<String>,
    #[serde(rename = "clientKeyData")]
    pub client_key_data: Option<String>,
}

pub async fn get_credentials(
    c: Client<AnyService>,
    opts: CredentialsOpts,
) -> Result<ExecCredential> {
    let res = c
        .auth()
        .cached_token_exchange(TokenExchangeRequest {
            header: Some(RequestHeader {
                profile: opts.profile,
            }),
            client_id: opts.client_id,
        })
        .await
        .wrap_err("failed to exchange token")?
        .into_inner();
    assert_response_valid(res.header).map_err(|e| eyre::eyre!("{e}"))?;

    let expiry: DateTime<Utc> = Utc::now() + TimeDelta::seconds(res.expires_in);

    Ok(ExecCredential {
        api_version: Some("client.authentication.k8s.io/v1".to_string()),
        kind: Some("ExecCredential".to_string()),
        status: Some(ExecCredentialStatus {
            client_certificate_data: None,
            client_key_data: None,
            token: Some(res.access_token),
            expiration_timestamp: Some(expiry),
        }),
    })
}
