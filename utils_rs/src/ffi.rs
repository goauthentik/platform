use cxx::CxxString;
use std::pin::Pin;

use crate::generated::agent::{Token};
use crate::generated::grpc_request;
use crate::generated::ping::ping_client::PingClient;
use crate::generated::pam::{TokenAuthResponse, TokenAuthRequest};

#[cxx::bridge]
mod ffi {
    extern "Rust" {
        type Token;
        type TokenAuthRequest;
        type TokenAuthResponse;

        fn grpc_ping(res: Pin<&mut CxxString>);
    }
}

fn grpc_ping(res: Pin<&mut CxxString>) {
    let resp = match grpc_request(async |ch| {
        return Ok(PingClient::new(ch)
            .ping(())
            .await?);
    }) {
        Ok(r) => r.into_inner().version,
        Err(e) => e.to_string(),
    };
    res.push_str(&resp);
}
