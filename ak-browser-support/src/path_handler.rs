use eyre::Result;
use ak_platform::{
    client::{
        sysd,
        user::{self, AnyService},
    },
    paths::SysdSocketID,
};

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
    pub async fn new() -> Result<Self> {
        let system_client = sysd::Client::new(SysdSocketID::Default)
            .await
            .map_err(|e| eyre::eyre!("failed to connect to system daemon: {e}"))?;
        let user_client = match user::Client::new(None).await {
            Ok(c) => Some(c),
            Err(e) => {
                tracing::warn!("failed to connect to user agent: {e:?}");
                None
            }
        };

        Ok(Self {
            system_client,
            user_client,
        })
    }

    pub async fn start(self) -> std::result::Result<(), NmError> {
        event_loop(move |raw: String, send: Sender| {
            let sself = self.clone();
            async move {
                let incoming: Message =
                    serde_json::from_str(&raw).map_err(NmError::DeserializeJson)?;
                tracing::debug!(path = incoming.route_path(), "Handling browser message");
                if incoming.version == "1" {
                    let res = sself.handle_v1(incoming).await?;
                    return send.send(&res).await;
                }
                tracing::warn!(
                    "Invalid version message received: {} (path {})",
                    incoming.version,
                    incoming.route_path()
                );
                Err(NmError::Disconnected)
            }
        })
        .await
    }

    async fn handle_v1(self, msg: Message) -> std::result::Result<Response, NmError> {
        let mm = msg.clone();
        let result = match mm.route_path().trim() {
            "ping" => self.handle_ping(mm).await,
            "get_token" => self.handle_get_token(mm).await,
            "list_profiles" => self.handle_list_profiles(mm).await,
            "platform_sign_endpoint_header" => self.handle_platform_sign_endpoint_header(mm).await,
            _ => Err(eyre::eyre!("No handler found")),
        };
        match result {
            Ok(res) => Ok(res),
            Err(e) => {
                tracing::warn!("Failed to run handler: {e:?}");
                Ok(Response::error_response(msg, e.to_string()))
            }
        }
    }
}
