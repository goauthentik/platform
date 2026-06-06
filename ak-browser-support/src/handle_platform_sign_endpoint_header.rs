use std::error::Error;

use authentik_sys::generated::{
    agent::RequestHeader,
    agent_platform::{PlatformEndpointRequest, agent_platform_client::AgentPlatformClient},
};
use serde_json::Value;

use crate::{
    models::{Message, Response},
    path_handler::PathHandler,
};

impl PathHandler {
    pub async fn handle_platform_sign_endpoint_header(
        &self,
        msg: Message,
    ) -> Result<Response, Box<dyn Error>> {
        let challenge = match msg.data.get("challenge") {
            Some(ch) => match ch.as_str() {
                Some(sch) => sch,
                None => return Err(Box::from("No challenge")),
            },
            None => return Err(Box::from("No challenge")),
        };
        let signed_response = AgentPlatformClient::new(self.system_channel.clone())
            .signed_endpoint_header(PlatformEndpointRequest {
                header: Some(RequestHeader {
                    profile: msg.profile.clone(),
                }),
                challenge: challenge.to_string(),
            })
            .await?
            .into_inner();
        let mut res = Response::in_response_to(msg);
        res.data.insert(
            "response".to_owned(),
            Value::String(signed_response.message),
        );
        Ok(res)
    }
}
