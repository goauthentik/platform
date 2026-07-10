use eyre::{Result, WrapErr};

use ak_platform::generated::{agent::RequestHeader, agent_platform::PlatformEndpointRequest};
use serde_json::Value;

use crate::{
    models::{Message, Response},
    path_handler::PathHandler,
};

impl PathHandler {
    pub async fn handle_platform_sign_endpoint_header(&self, msg: Message) -> Result<Response> {
        let challenge = msg
            .data
            .get("challenge")
            .and_then(|ch| ch.as_str())
            .ok_or_else(|| eyre::eyre!("No challenge"))?;
        let signed_response = self
            .system_client
            .clone()
            .platform()
            .signed_endpoint_header(PlatformEndpointRequest {
                header: Some(RequestHeader {
                    profile: msg.profile.clone(),
                }),
                challenge: challenge.to_string(),
            })
            .await
            .wrap_err("failed to sign endpoint header")?
            .into_inner();
        let mut res = Response::in_response_to(msg);
        res.data.insert(
            "response".to_owned(),
            Value::String(signed_response.message),
        );
        Ok(res)
    }
}
