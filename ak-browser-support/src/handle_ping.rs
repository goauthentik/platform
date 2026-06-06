use std::error::Error;

use native_messaging::host::Sender;

use crate::{
    models::{Message, Response},
    path_handler::PathHandler,
};

impl PathHandler {
    pub async fn handle_ping(&self, msg: Message, send: Sender) -> Result<(), Box<dyn Error>> {
        let mut a = Response::in_response_to(msg);
        a.data.insert(
            "ping".to_owned(),
            serde_json::Value::String("pong".to_owned()),
        );
        send.send(&a).await?;
        Ok(())
    }
}
