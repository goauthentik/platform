use eyre::{Result, WrapErr};

use ak_platform::generated::{
    agent::RequestHeader,
    agent_auth::{CurrentTokenRequest, current_token_request},
};

use crate::{
    models::{Message, Response},
    path_handler::PathHandler,
};

impl PathHandler {
    pub async fn handle_get_token(&self, msg: Message) -> Result<Response> {
        let uc = self
            .user_client
            .as_ref()
            .ok_or_else(|| eyre::eyre!("Not connected to user agent"))?
            .clone();
        let current = uc
            .auth()
            .get_current_token(CurrentTokenRequest {
                header: Some(RequestHeader {
                    profile: msg.profile.clone(),
                }),
                r#type: current_token_request::Type::Verified as i32,
            })
            .await
            .wrap_err("failed to get current token")?
            .into_inner();
        let mut res = Response::in_response_to(msg);
        res.data
            .insert("token".to_owned(), serde_json::Value::String(current.raw));
        res.data
            .insert("url".to_owned(), serde_json::Value::String(current.url));
        Ok(res)
    }
}
