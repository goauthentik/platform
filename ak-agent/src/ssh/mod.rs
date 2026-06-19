use std::sync::Arc;

use crate::Agent;
use ak_platform::{
    net::server::{ConnectedLocalStream, ListenerStream, SocketPermMode, listen},
    paths::{AgentSocketID, agent_socket_path},
    prelude::*,
};
use ssh_agent_lib::agent::{Session, listen as ssh_listen};
use ssh_key::{Algorithm, PrivateKey, rand_core::OsRng};
use tonic::transport::server::Connected as _;

pub mod agent;
pub mod ext_ak;
pub mod ext_session_bind;
pub mod txn;
pub mod txn_keys;

use txn::SSHAgentTransaction;
use uuid::Uuid;

#[derive(Clone)]
pub struct AgentSSHServer {
    agent: Arc<Agent>,
    priv_key: Arc<PrivateKey>,
    profile: String,
}

impl ssh_agent_lib::agent::Agent<ListenerStream> for AgentSSHServer {
    fn new_session(&mut self, socket: &ConnectedLocalStream) -> impl Session {
        SSHAgentTransaction {
            agent: Arc::clone(&self.agent),
            priv_key: Arc::clone(&self.priv_key),
            profile: self.profile.clone(),
            creds: socket.connect_info(),
            host_key: None,
            session_id: None,
            cert: None,
            id: Uuid::new_v4(),
        }
    }
}

impl AgentSSHServer {
    pub async fn new(agent: Arc<Agent>) -> Result<Self> {
        let priv_key = PrivateKey::random(&mut OsRng, Algorithm::Ed25519)
            .map_err(|e| -> BoxError { Box::from(e.to_string()) })?;

        let profile = agent
            .cfg
            .read()
            .await
            .profiles
            .keys()
            .next()
            .cloned()
            .unwrap_or("default".into());

        Ok(AgentSSHServer {
            agent,
            priv_key: Arc::new(priv_key),
            profile,
        })
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
                tracing::warn!("failed to listen: {e:?}");
                return Err(e);
            }
        };
        ssh_listen(listener, self).await?;
        Ok(())
    }
}
