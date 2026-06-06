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
