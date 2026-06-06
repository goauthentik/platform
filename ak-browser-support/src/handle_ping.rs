use std::error::Error;

use crate::{
    models::{Message, Response},
    path_handler::PathHandler,
};

impl PathHandler {
    pub async fn handle_ping(&self, msg: Message) -> Result<Response, Box<dyn Error>> {
        let mut res = Response::in_response_to(msg);
        res.data.insert(
            "ping".to_owned(),
            serde_json::Value::String("pong".to_owned()),
        );
        Ok(res)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_ping() {
        let handler = PathHandler {
            system_channel: None,
            user_channel: None,
        };
        let res = handler.handle_ping(Message::test_msg()).await.unwrap();
        assert_eq!(res.data.get("ping").unwrap().as_str().unwrap(), "pong");
    }
}
