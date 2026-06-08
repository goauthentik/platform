use std::error::Error;

use ak_platform::{
    generated::{
        agent::RequestHeader,
        agent_auth::{WhoAmIRequest, agent_auth_client::AgentAuthClient},
    },
    grpc::{assert_response_valid, grpc_endpoint},
    platform::paths::{AgentSocketID, agent_socket_path},
};

use crate::{Cli, format};

pub async fn whoami(cli: &Cli) -> Result<(), Box<dyn Error>> {
    let c = grpc_endpoint(agent_socket_path(AgentSocketID::Default)?.for_current()).await?;
    let res = AgentAuthClient::new(c)
        .who_am_i(WhoAmIRequest {
            header: Some(RequestHeader {
                profile: cli.profile.clone(),
            }),
        })
        .await?
        .into_inner();
    assert_response_valid(res.header)?;
    format::render_json(res.body, "User Information", cli.json)
}
