use cxx::{CxxString, let_cxx_string};
use std::collections::HashMap;
use std::error::Error;
use std::pin::Pin;
use url::Url;

use crate::generated::ping::ping_client::PingClient;
use crate::generated::sys_auth::TokenAuthRequest;
use crate::generated::sys_auth::system_auth_interactive_client::SystemAuthInteractiveClient;
use crate::generated::sys_auth::system_auth_token_client::SystemAuthTokenClient;
use crate::grpc::grpc_request;

const TOKEN_QUERY_PARAM: &str = "ak-auth-ia-token";

#[cxx::bridge]
#[allow(clippy::module_inception)]
mod ffi {
    struct WCPAuthStartAsync {
        pub url: String,
        pub header_token: String,
    }

    struct TokenResponse {
        pub username: String,
        pub session_id: String,
    }

    extern "Rust" {
        fn ak_sys_ping(res: Pin<&mut CxxString>);

        fn ak_sys_auth_url(url: &CxxString, token: &mut TokenResponse) -> Result<bool>;
        fn ak_sys_auth_token_validate(
            raw_token: &CxxString,
            token: &mut TokenResponse,
        ) -> Result<bool>;
        fn ak_sys_auth_start_async(res: &mut WCPAuthStartAsync) -> Result<bool>;
    }
}

fn ak_sys_ping(res: Pin<&mut CxxString>) {
    let resp = match grpc_request(async |ch| {
        return Ok(PingClient::new(ch).ping(()).await?);
    }) {
        Ok(r) => r.into_inner().version,
        Err(e) => e.to_string(),
    };
    res.push_str(&resp);
}

fn ak_sys_auth_url(
    url: &CxxString,
    token: &mut ffi::TokenResponse,
) -> Result<bool, Box<dyn Error>> {
    let p = Url::parse(url.to_str()?)?;
    let qm: HashMap<_, _> = p.query_pairs().into_owned().collect();
    let raw_token = qm
        .get(TOKEN_QUERY_PARAM)
        .ok_or("failed to get token from URL")?;
    let_cxx_string!(crt = raw_token);
    ak_sys_auth_token_validate(&crt, token)
}

fn ak_sys_auth_token_validate(
    raw_token: &CxxString,
    token: &mut ffi::TokenResponse,
) -> Result<bool, Box<dyn Error>> {
    let p = raw_token.to_str()?;
    let response = grpc_request(async |ch| {
        return Ok(SystemAuthTokenClient::new(ch)
            .token_auth(TokenAuthRequest {
                username: "".to_string(),
                token: p.to_owned(),
            })
            .await?);
    })?
    .into_inner();

    if let Some(pt) = response.token {
        token.username = pt.preferred_username;
    }
    token.session_id = response.session_id;
    Ok(response.successful)
}

fn ak_sys_auth_start_async(res: &mut ffi::WCPAuthStartAsync) -> Result<bool, Box<dyn Error>> {
    let response = grpc_request(async |ch| {
        return Ok(SystemAuthInteractiveClient::new(ch)
            .interactive_auth_async(())
            .await?);
    })?
    .into_inner();
    res.url = response.url;
    res.header_token = response.header_token;
    Ok(true)
}
