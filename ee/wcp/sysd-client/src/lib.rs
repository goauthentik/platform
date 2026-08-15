//! The three `ak-sysd` gRPC calls the credential provider needs:
//! capability discovery, starting an interactive sign-in, and validating the
//! token the sign-in redirects back with.
//!
//! This used to be the root workspace's `ak_ffi` crate, which existed to
//! expose these over a `#[cxx::bridge]` to the old C++ provider. With that
//! tree gone, `credprovider`/`cef-host` are the only callers left, so it
//! lives here as an ordinary Rust library.

use eyre::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use url::Url;
use winreg::enums::HKEY_LOCAL_MACHINE;

use ak_platform::generated::ping::capabilities_response::Capability;
use ak_platform::generated::ping::ping_client::PingClient;
use ak_platform::generated::sys_auth::TokenAuthRequest;
use ak_platform::generated::sys_auth::system_auth_interactive_client::SystemAuthInteractiveClient;
use ak_platform::generated::sys_auth::system_auth_token_client::SystemAuthTokenClient;
use ak_platform::grpc::grpc_request;

use wire::TOKEN_QUERY_PARAM;

/// HKLM key `sys_caps` caches its answer in, so a second call doesn't need
/// `ak-sysd` to be reachable. Public so `e2e` can seed it.
pub const CAPABILITIES_KEY: &str = "SOFTWARE\\authentik Security Inc.\\Platform\\Capabilities";

pub struct AuthStartAsync {
    pub url: String,
    pub header_token: String,
}

pub struct TokenResponse {
    pub username: String,
    pub session_id: String,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct Capabilities {
    pub interactive_auth_available: bool,
    pub debug: bool,
}

pub fn sys_caps() -> Result<Capabilities> {
    let hklm = winreg::RegKey::predef(HKEY_LOCAL_MACHINE);
    let (key, _disp) = hklm.create_subkey(CAPABILITIES_KEY)?;

    if let Ok(caps) = key.decode() {
        return Ok(caps);
    }

    let response =
        grpc_request(async |ch| Ok(PingClient::new(ch).capabilities(()).await?))?.into_inner();
    let authia = Capability::AuthInteractive as i32;
    let caps = Capabilities {
        interactive_auth_available: response.capabilities.contains(&authia),
        debug: false,
    };
    key.encode(&caps)?;
    Ok(caps)
}

pub fn sys_auth_start_async() -> Result<AuthStartAsync> {
    let response = grpc_request(async |ch| {
        Ok(SystemAuthInteractiveClient::new(ch)
            .interactive_auth_async(())
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
        session_id: response.session_id,
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
            wire::REDIRECT_PREFIX
        );
        assert_eq!(extract_token(&url).unwrap(), "abc123");
    }

    #[test]
    fn extracts_token_alongside_other_query_params() {
        let url = format!(
            "{}callback?state=xyz&{TOKEN_QUERY_PARAM}=abc123&code=9",
            wire::REDIRECT_PREFIX
        );
        assert_eq!(extract_token(&url).unwrap(), "abc123");
    }

    #[test]
    fn errors_when_the_token_param_is_absent() {
        let url = format!("{}callback?state=xyz", wire::REDIRECT_PREFIX);
        assert!(extract_token(&url).is_err());
    }

    #[test]
    fn errors_on_an_unparseable_url() {
        assert!(extract_token("not a url").is_err());
    }
}
