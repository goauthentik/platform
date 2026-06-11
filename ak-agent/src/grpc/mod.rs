use std::error::Error;

use ak_platform::{generated::agent_auth::agent_auth_server::AgentAuthServer, net::server::{ListenerStream, SocketPermMode, listen}, paths::{AgentSocketID, agent_socket_path}};
use tonic::transport::{Server, server::Router};

use crate::Agent;

pub mod agent_auth;

pub struct AgentGRPCServer {
    agent: Agent,
}

impl AgentGRPCServer {

    pub async fn new(agent: Agent) -> Result<AgentGRPCServer, Box<dyn Error + Send>> {
        let ag =AgentGRPCServer { agent };
        Ok(ag)
    }

    pub async fn start(self) -> Result<(), Box<dyn Error + Send>> {
        let listener = match listen(
            agent_socket_path(AgentSocketID::Default)?,
            SocketPermMode::Owner,
        )
        .await {
            Ok(l) => l,
            Err(e) => {
                log::warn!("failed to listen: {e:?}");
                return Err(e)
            },
        };
        Ok(Server::builder()
            .add_service(AgentAuthServer::new(self))
            .serve_with_incoming(listener)
            .await?)
    }

}
