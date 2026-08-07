use reqwest::Client;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct TokenResponse {
    pub access_token: String,
    issued_token_type: String,
    token_type: String,
    #[serde(default)]
    expires_in: Option<u64>,
    #[serde(default)]
    scope: Option<String>,
    #[serde(default)]
    refresh_token: Option<String>,
}

pub async fn token_exchange(
    client: &Client,
    token_endpoint: &str,
    client_id: &str,
    client_secret: &str,
    subject_token: &str,
    actor_tolen: &str,
    audience: &str,
) -> reqwest::Result<TokenResponse> {
    let params = [
        ("grant_type", "urn:ietf:params:oauth:grant-type:token-exchange"),
        ("subject_token", subject_token),
        ("subject_token_type", "urn:ietf:params:oauth:token-type:access_token"),
        ("audience", audience),
        ("actor_token", actor_tolen),
        ("actor_token_type", "goauthentik.io/oauth/token-type/authentik_token"),
    ];

    client
        .post(token_endpoint)
        .basic_auth(client_id, Some(client_secret))
        .form(&params)
        .send()
        .await?
        .error_for_status()?
        .json::<TokenResponse>()
        .await
}
