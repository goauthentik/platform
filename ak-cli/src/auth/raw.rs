use std::error::Error;

use ak_platform::{
    generated::{
        agent::RequestHeader,
        agent_auth::{TokenExchangeRequest, agent_auth_client::AgentAuthClient},
    },
    grpc::{assert_response_valid, grpc_endpoint},
    platform::paths::{AgentSocketID, agent_socket_path},
};

pub struct CredentialsOpts {
    pub profile: String,
    pub client_id: String,
}

pub struct RawCredentialOutput {
    pub access_token: String,
}

pub async fn get_credentials(opts: CredentialsOpts) -> Result<RawCredentialOutput, Box<dyn Error>> {
    let c = grpc_endpoint(agent_socket_path(AgentSocketID::Default)?.for_current()).await?;
    let res = AgentAuthClient::new(c)
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
