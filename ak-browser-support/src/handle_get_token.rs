use std::error::Error;

use ak_platform::generated::{
    agent::RequestHeader,
    agent_auth::{CurrentTokenRequest, current_token_request},
};

use crate::{
    models::{Message, Response},
    path_handler::PathHandler,
};

impl PathHandler {
    pub async fn handle_get_token(&self, msg: Message) -> Result<Response, Box<dyn Error>> {
        let uc = match &self.user_client {
            Some(c) => c.clone(),
            None => return Err(Box::from("Not connected to user agent")),
        };
        let current = uc
            .auth()
            .get_current_token(CurrentTokenRequest {
                header: Some(RequestHeader {
                    profile: msg.profile.clone(),
                }),
                r#type: current_token_request::Type::Verified as i32,
            })
            .await?
            .into_inner();
        let mut res = Response::in_response_to(msg);
        res.data
            .insert("token".to_owned(), serde_json::Value::String(current.raw));
        res.data
            .insert("url".to_owned(), serde_json::Value::String(current.url));
        Ok(res)
    }
}
