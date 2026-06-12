use std::sync::Arc;

use crate::Agent;
use ak_platform::{
    net::server::{ConnectedLocalStream, ListenerStream, SocketPermMode, listen},
    paths::{AgentSocketID, agent_socket_path},
    prelude::*,
};
use ssh_agent_lib::agent::{Session, listen as ssh_listen};

pub mod agent;

#[derive(Clone)]
pub struct AgentSSHServer {
    _agent: Arc<Agent>,
}

impl ssh_agent_lib::agent::Agent<ListenerStream> for AgentSSHServer {
    fn new_session(&mut self, _socket: &ConnectedLocalStream) -> impl Session {
        self.clone()
    }
}

impl AgentSSHServer {
    pub async fn new(agent: Arc<Agent>) -> Self {
        AgentSSHServer { _agent: agent }
    }
    pub async fn start(self) -> Result<()> {
        let listener = match listen(
            agent_socket_path(AgentSocketID::SSH)?,
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
        ssh_listen(listener, self).await?;
        Ok(())
    }
}
