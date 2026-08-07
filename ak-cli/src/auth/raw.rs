use ak_platform::{
    client::user::{AnyService, Client},
    generated::{agent::RequestHeader, agent_auth::TokenExchangeRequest},
    grpc::assert_response_valid,
};
use eyre::{Result, WrapErr};

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
) -> Result<RawCredentialOutput> {
    let res = c
        .auth()
        .cached_token_exchange(TokenExchangeRequest {
            header: Some(RequestHeader {
                profile: opts.profile,
            }),
            audience: opts.client_id,
            scopes: vec![],
            actor_token: None,
            actor_token_type: None,
        })
        .await
        .wrap_err("failed to exchange token")?
        .into_inner();
    assert_response_valid(res.header)?;
    Ok(RawCredentialOutput {
        access_token: res.access_token,
    })
}
