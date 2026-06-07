use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Deserialize, Clone)]
pub(crate) struct Message {
    pub version: String,
    path: String,
    id: String,

    pub profile: String,
    pub data: HashMap<String, Value>,
}

#[cfg(test)]
impl Message {
    pub fn test_msg() -> Message {
        return Message {
            version: "1".to_string(),
            path: "".to_string(),
            id: "foo".to_string(),
            profile: "".to_string(),
            data: HashMap::new(),
        };
    }
}

impl Message {
    pub fn route_path(&self) -> String {
        self.path.clone()
    }
    pub fn message_id(&self) -> String {
        self.id.clone()
    }
}

#[derive(Serialize, Clone)]
pub(crate) struct Response {
    pub data: HashMap<String, Value>,
    response_to: String,
}

impl Response {
    pub fn in_response_to(m: Message) -> Response {
        Response {
            data: HashMap::new(),
            response_to: m.message_id().clone(),
        }
    }
}
