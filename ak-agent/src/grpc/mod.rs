use std::sync::Arc;

use crate::Agent;
use ak_platform::generated::agent_cache::agent_cache_server::AgentCacheServer;
use ak_platform::generated::agent_ctrl::agent_ctrl_server::AgentCtrlServer;
use ak_platform::generated::ping::ping_server::PingServer;
use ak_platform::prelude::*;
use ak_platform::{
    generated::agent_auth::agent_auth_server::AgentAuthServer,
    net::server::{SocketPermMode, listen},
    paths::{AgentSocketID, agent_socket_path},
};
use tonic::transport::Server;

pub mod agent_auth;
pub mod agent_cache;
pub mod agent_ctrl;
pub mod ping;

pub struct AgentGRPCServer {
    agent: Agent,
}

impl AgentGRPCServer {
    pub async fn new(agent: Agent) -> Result<AgentGRPCServer> {
        Ok(AgentGRPCServer { agent })
    }

    pub async fn start(self) -> Result<()> {
        let listener = match listen(
            agent_socket_path(AgentSocketID::Default)?,
            SocketPermMode::Owner,
        )
        .await
        {
            Ok(l) => l,
            Err(e) => {
                log::warn!("failed to listen: {e:?}");
                return Err(e);
            }
        };
        let shared = Arc::new(self);
        Ok(Server::builder()
            .add_service(AgentAuthServer::from_arc(Arc::clone(&shared)))
            .add_service(AgentCacheServer::from_arc(Arc::clone(&shared)))
            .add_service(AgentCtrlServer::from_arc(Arc::clone(&shared)))
            .add_service(PingServer::from_arc(Arc::clone(&shared)))
            .serve_with_incoming(listener)
            .await?)
    }
}
