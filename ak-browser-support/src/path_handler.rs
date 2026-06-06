use authentik_sys::{
    grpc::grpc_endpoint,
    platform::paths::{AgentSocketID, SysdSocketID, agent_socket_path, sysd_socket_path},
};
use std::error::Error;
use tonic::transport::Channel;

use native_messaging::{
    event_loop,
    host::{NmError, Sender},
};

use crate::models::{Message, Response};

#[derive(Clone)]
pub(crate) struct PathHandler {
    pub(crate) system_channel: Channel,
    pub(crate) user_channel: Channel,
}

impl PathHandler {
    pub async fn new() -> Result<Self, Box<dyn Error>> {
        let sys_channel =
            grpc_endpoint(sysd_socket_path(SysdSocketID::Default).for_current()).await?;
        let user_channel =
            grpc_endpoint(agent_socket_path(AgentSocketID::Default).for_current()).await?;

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
