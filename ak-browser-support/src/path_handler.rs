use ak_platform::{
    client::{sysd, user::{self, AnyService}},
    platform::paths::SysdSocketID,
};
use std::error::Error;

use native_messaging::{
    event_loop,
    host::{NmError, Sender},
};

use crate::models::{Message, Response};

#[derive(Clone)]
pub(crate) struct PathHandler {
    pub(crate) system_client: sysd::Client,
    pub(crate) user_client: Option<user::Client<AnyService>>,
}

impl PathHandler {
    pub async fn new() -> Result<Self, Box<dyn Error>> {
        let system_client = sysd::Client::new(SysdSocketID::Default).await?;
        let user_client =
            match user::Client::new().await {
                Ok(c) => Some(c),
                Err(e) => {
                    log::warn!("failed to connect to user agent: {e:?}");
                    None
                }
            };

        Ok(Self {
            system_client,
            user_client,
        })
    }

    pub async fn start(self) -> Result<(), NmError> {
        event_loop(move |raw: String, send: Sender| {
            let sself = self.clone();
            async move {
                let incoming: Message =
                    serde_json::from_str(&raw).map_err(NmError::DeserializeJson)?;
                log::debug!("Handling browser message {}", incoming.route_path());
                if incoming.version == "1" {
                    let res = sself.handle_v1(incoming).await?;
                    return send.send(&res).await;
                }
                log::warn!(
                    "Invalid version message received: {} (path {})",
                    incoming.version,
                    incoming.route_path()
                );
                Err(NmError::Disconnected)
            }
        })
        .await
    }

    async fn handle_v1(self, msg: Message) -> Result<Response, NmError> {
        let result = match msg.route_path().trim() {
            "ping" => self.handle_ping(msg).await,
            "get_token" => self.handle_get_token(msg).await,
            "list_profiles" => self.handle_list_profiles(msg).await,
            "platform_sign_endpoint_header" => self.handle_platform_sign_endpoint_header(msg).await,
            _ => Err(Box::from("No handler found")),
        };
        match result {
            Ok(res) => Ok(res),
            Err(e) => {
                log::warn!("Failed to run handler: {e:?}");
                Err(NmError::Disconnected)
            }
        }
    }
}
