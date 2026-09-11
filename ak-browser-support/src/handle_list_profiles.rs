use eyre::{Result, WrapErr};

use serde_json::{Value, to_value};

use crate::{
    models::{Message, Response},
    path_handler::PathHandler,
};

impl PathHandler {
    pub async fn handle_list_profiles(&self, msg: Message) -> Result<Response> {
        let uc = self
            .user_client
            .as_ref()
            .ok_or_else(|| eyre::eyre!("Not connected to user agent"))?
            .clone();
        let profiles = uc
            .ctrl()
            .list_profiles(())
            .await
            .wrap_err("failed to list profiles")?
            .into_inner();
        let mut res = Response::in_response_to(msg);
        let c = profiles
            .profiles
            .iter()
            .map(|p| to_value(p).unwrap_or(Value::Null))
            .collect();
        res.data.insert("profiles".to_owned(), Value::Array(c));
        Ok(res)
    }
}
