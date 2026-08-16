//! The two `ak-sysd` calls the browser host makes: start an interactive
//! sign-in (yielding the URL to open and the header token to inject), and
//! validate the token the `goauthentik.io://` redirect returns.

use eyre::Result;
use std::collections::HashMap;
use url::Url;

use ak_ee_wcp_wire::TOKEN_QUERY_PARAM;
use ak_platform::generated::sys_auth::system_auth_interactive_client::SystemAuthInteractiveClient;
use ak_platform::generated::sys_auth::system_auth_token_client::SystemAuthTokenClient;
use ak_platform::generated::sys_auth::{InteractiveAuthAsyncRequest, TokenAuthRequest};
use ak_platform::grpc::grpc_request;

pub struct AuthStartAsync {
    pub url: String,
    pub header_token: String,
}

pub struct TokenResponse {
    pub username: String,
}

/// `login_hint` is the username of the tile that was selected, so the sign-in
/// page can skip the identification stage. It is only ever a hint: what the
/// flow authenticates as is whatever `sys_auth_url` reports back.
pub fn sys_auth_start_async(login_hint: Option<String>) -> Result<AuthStartAsync> {
    let response = grpc_request(async |ch| {
        Ok(SystemAuthInteractiveClient::new(ch)
            .interactive_auth_async(InteractiveAuthAsyncRequest {
                username: login_hint.clone(),
            })
            .await?)
    })?
    .into_inner();
    Ok(AuthStartAsync {
        url: response.url,
        header_token: response.header_token,
    })
}

pub fn sys_auth_url(url: &str) -> Result<Option<TokenResponse>> {
    let raw_token = extract_token(url)?;
    sys_auth_token_validate(&raw_token)
}

fn extract_token(url: &str) -> Result<String> {
    let parsed = Url::parse(url)?;
    let qm: HashMap<_, _> = parsed.query_pairs().into_owned().collect();
    qm.get(TOKEN_QUERY_PARAM)
        .cloned()
        .ok_or_else(|| eyre::eyre!("failed to get token from URL"))
}

fn sys_auth_token_validate(raw_token: &str) -> Result<Option<TokenResponse>> {
    let response = grpc_request(async |ch| {
        Ok(SystemAuthTokenClient::new(ch)
            .token_auth(TokenAuthRequest {
                username: String::new(),
                token: raw_token.to_owned(),
            })
            .await?)
    })?
    .into_inner();

    if !response.successful {
        return Ok(None);
    }
    Ok(Some(TokenResponse {
        username: response
            .token
            .map(|t| t.preferred_username)
            .unwrap_or_default(),
    }))
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn extracts_token_from_a_redirect_url() {
        let url = format!(
            "{}callback?{TOKEN_QUERY_PARAM}=abc123",
            ak_ee_wcp_wire::REDIRECT_PREFIX
        );
        assert_eq!(extract_token(&url).unwrap(), "abc123");
    }

    #[test]
    fn extracts_token_alongside_other_query_params() {
        let url = format!(
            "{}callback?state=xyz&{TOKEN_QUERY_PARAM}=abc123&code=9",
            ak_ee_wcp_wire::REDIRECT_PREFIX
        );
        assert_eq!(extract_token(&url).unwrap(), "abc123");
    }

    #[test]
    fn errors_when_the_token_param_is_absent() {
        let url = format!("{}callback?state=xyz", ak_ee_wcp_wire::REDIRECT_PREFIX);
        assert!(extract_token(&url).is_err());
    }

    #[test]
    fn errors_on_an_unparseable_url() {
        assert!(extract_token("not a url").is_err());
    }
}
