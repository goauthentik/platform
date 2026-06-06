use std::error::Error;

use authentik_sys::generated::agent_ctrl::agent_ctrl_client::AgentCtrlClient;
use native_messaging::host::Sender;
use serde_json::{Value, to_value};

use crate::{
    models::{Message, Response},
    path_handler::PathHandler,
};

impl PathHandler {
    pub async fn handle_list_profiles(
        &self,
        msg: Message,
        send: Sender,
    ) -> Result<(), Box<dyn Error>> {
        let profiles = AgentCtrlClient::new(self.user_channel.clone())
            .list_profiles(())
            .await?
            .into_inner();
        let mut res = Response::in_response_to(msg);
        let c = profiles
            .profiles
            .iter()
            .map(|p| to_value(p).unwrap_or(Value::Null))
            .collect();
        res.data.insert("profiles".to_owned(), Value::Array(c));
        send.send(&res).await?;
        Ok(())
    }
}
