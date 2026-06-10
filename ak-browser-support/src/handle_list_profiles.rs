use std::error::Error;

use serde_json::{Value, to_value};

use crate::{
    models::{Message, Response},
    path_handler::PathHandler,
};

impl PathHandler {
    pub async fn handle_list_profiles(&self, msg: Message) -> Result<Response, Box<dyn Error>> {
        let uc = match &self.user_client {
            Some(c) => c.clone(),
            None => return Err(Box::from("Not connected to user agent")),
        };
        let profiles = uc.ctrl().list_profiles(())
            .await?
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
