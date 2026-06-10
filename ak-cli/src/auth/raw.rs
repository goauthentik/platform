use std::error::Error;

use ak_platform::{
    client::user::{AnyService, Client},
    generated::{
        agent::RequestHeader,
        agent_auth::TokenExchangeRequest,
    },
    grpc::assert_response_valid,
};

pub struct CredentialsOpts {
    pub profile: String,
    pub client_id: String,
}

pub struct RawCredentialOutput {
    pub access_token: String,
}

pub async fn get_credentials(
    c: Client<AnyService>,
    opts: CredentialsOpts,
) -> Result<RawCredentialOutput, Box<dyn Error>> {
    let res = c
        .auth()
        .cached_token_exchange(TokenExchangeRequest {
            header: Some(RequestHeader {
                profile: opts.profile,
            }),
            client_id: opts.client_id,
        })
        .await?
        .into_inner();
    assert_response_valid(res.header)?;
    Ok(RawCredentialOutput {
        access_token: res.access_token,
    })
}
