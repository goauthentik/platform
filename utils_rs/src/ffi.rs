use cxx::CxxString;
use std::error::Error;
use std::pin::Pin;

use crate::generated::grpc_request;
use crate::generated::ping::ping_client::PingClient;
use crate::generated::sys_auth::TokenAuthRequest;
use crate::generated::sys_auth::system_auth_token_client::SystemAuthTokenClient;

#[cxx::bridge]
mod ffi {
    struct WCPOAuthConfig {
        pub url: String,
        pub client_id: String,
    }

    struct TokenResponse {
        pub username: String,
        pub session_id: String,
    }

    extern "Rust" {
        fn ak_sys_grpc_ping(res: Pin<&mut CxxString>);
        fn ak_sys_token_validate(
            username: &CxxString,
            raw_token: &CxxString,
            token: &mut TokenResponse,
        ) -> Result<bool>;

        fn ak_sys_wcp_oauth_config(res: &mut WCPOAuthConfig) -> Result<bool>;
    }
}

fn ak_sys_grpc_ping(res: Pin<&mut CxxString>) {
    let resp = match grpc_request(async |ch| {
        return Ok(PingClient::new(ch).ping(()).await?);
    }) {
        Ok(r) => r.into_inner().version,
        Err(e) => e.to_string(),
    };
    res.push_str(&resp);
}

fn ak_sys_token_validate(
    username: &CxxString,
    raw_token: &CxxString,
    token: &mut ffi::TokenResponse,
) -> Result<bool, Box<dyn Error>> {
    let u = username.to_str()?;
    let p = raw_token.to_str()?;
    let response = grpc_request(async |ch| {
        return Ok(SystemAuthTokenClient::new(ch)
            .token_auth(TokenAuthRequest {
                username: u.to_owned(),
                token: p.to_owned(),
            })
            .await?);
    })?
    .into_inner();

    token.username = response.token.unwrap().preferred_username;
    token.session_id = response.session_id;
    Ok(response.successful)
}

fn ak_sys_wcp_oauth_config(res: &mut ffi::WCPOAuthConfig) -> Result<bool, Box<dyn Error>> {
    let response = grpc_request(async |ch| {
        return Ok(SystemAuthTokenClient::new(ch)
            .o_auth_params(()).await?);
    })?
    .into_inner();
    res.url = response.url;
    res.client_id = response.client_id;
    Ok(true)
}
