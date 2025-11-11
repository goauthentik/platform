use cxx::CxxString;
use std::error::Error;
use std::pin::Pin;

use crate::generated::agent::Token;
use crate::generated::grpc_request;
use crate::generated::ping::ping_client::PingClient;
use crate::generated::sys_auth::system_auth_token_client::SystemAuthTokenClient;
use crate::generated::sys_auth::{TokenAuthRequest, TokenAuthResponse};

#[cxx::bridge]
mod ffi {
    struct WCPOAuthConfig {
        pub url: String,
        pub client_id: String,
    }

    extern "Rust" {
        type Token;
        type TokenAuthRequest;
        type TokenAuthResponse;

        fn ak_sys_grpc_ping(res: Pin<&mut CxxString>);
        fn ak_sys_token_validate(username: &CxxString, token: &CxxString) -> Result<bool>;

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

fn ak_sys_token_validate(username: &CxxString, token: &CxxString) -> Result<bool, Box<dyn Error>> {
    let u = username.to_str()?;
    let p = token.to_str()?;
    let response = grpc_request(async |ch| {
        return Ok(SystemAuthTokenClient::new(ch)
            .token_auth(TokenAuthRequest {
                username: u.to_owned(),
                token: p.to_owned(),
            })
            .await?);
    })?
    .into_inner();

    Ok(response.successful)
}

fn ak_sys_wcp_oauth_config(res: &mut ffi::WCPOAuthConfig) -> Result<bool, Box<dyn Error>> {
    let config = ffi::WCPOAuthConfig {
        url: "https://windows-cred-provider.pr.test.goauthentik.io".to_string(),
        client_id: "UCAXCsLq1DVR08hYrjDGFPFekCVXmNTEn6eeoenO".to_string(),
    };
    res.url = config.url;
    res.client_id = config.client_id;
    Ok(true)
}
