use authentik_sys::grpc::grpc_endpoint;
use std::error::Error;
use tonic::transport::{Channel, Endpoint};

use native_messaging::{
    event_loop,
    host::{NmError, Sender},
};

use crate::models::Message;

#[derive(Clone)]
pub(crate) struct PathHandler {
    pub(crate) system_channel: Channel,
    pub(crate) user_channel: Channel,
}

impl PathHandler {
    pub async fn new() -> Result<Self, Box<dyn Error>> {
        log::debug!("Creating GRPC connection to sysd");
        let sys_ep = Endpoint::try_from(format!("http://:123/?{}", "path"))?;
        let sys_channel = grpc_endpoint(sys_ep).await?;
        log::debug!("Creating GRPC connection to user-agent");
        let user_ep = Endpoint::try_from(format!("http://:123/?{}", "path"))?;
        let user_channel = grpc_endpoint(user_ep).await?;

        Ok(Self {
            system_channel: sys_channel,
            user_channel,
        })
    }

    pub async fn start(self) -> Result<(), NmError> {
        event_loop(move |raw: String, send: Sender| {
            let sself = self.clone();
            async move {
                let incoming: Message =
                    serde_json::from_str(&raw).map_err(NmError::DeserializeJson)?;
                if incoming.version == "1" {
                    return sself.handle_v1(incoming, send).await;
                }
                log::warn!("Invalid version message received: {} (path {})", incoming.version, incoming.route_path());
                Err(NmError::Disconnected)
            }
        })
        .await
    }

    async fn handle_v1(self, msg: Message, send: Sender) -> Result<(), NmError> {
        let result = match msg.route_path().trim() {
            "ping" => self.handle_ping(msg, send).await,
            "get_token" => self.handle_get_token(msg, send).await,
            "list_profiles" => self.handle_list_profiles(msg, send).await,
            "platform_sign_endpoint_header" => {
                self
                    .handle_platform_sign_endpoint_header(msg, send)
                    .await
            }
            _ => Ok(()),
        };
        match result {
            Ok(_) => Ok(()),
            Err(e) => {
                log::warn!("Failed to run handler: {e:?}");
                Err(NmError::Disconnected)
            }
        }
    }
}
