use std::error::Error;

use native_messaging::host::Sender;

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

// #[cfg(test)]
// mod tests {
//     // Note this useful idiom: importing names from outer (for mod tests) scope.
//     use super::*;

//     #[test]
//     fn test_ping() {
//         let handler = PathHandler {
//             system_channel: None,
//             user_channel: None,
//         };
//         assert_eq!(handler.handle_ping(Message {}, Sender {}));
//     }

// }
