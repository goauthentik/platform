use std::collections::HashMap;
use std::sync::Arc;

use crate::Agent;
use ak_platform::{
    net::server::{ConnectedLocalStream, ListenerStream, SocketPermMode, listen},
    paths::{AgentSocketID, agent_socket_path},
};
use eyre::{Result, WrapErr};
use ssh_agent_lib::agent::{Session, listen as ssh_listen};
use ssh_key::{Algorithm, PrivateKey, rand_core::OsRng};
use tokio::task::JoinHandle;
use tonic::transport::server::Connected as _;

pub mod agent;
pub mod ext_ak;
pub mod ext_session_bind;
pub mod passthrough;
pub mod txn;
pub mod txn_keys;

use txn::SSHAgentTransaction;
use uuid::Uuid;

#[derive(Clone)]
pub struct AgentSSHServer {
    agent: Arc<Agent>,
    priv_key: Arc<PrivateKey>,
    /// Profile this server is pinned to; `None` follows the active profile.
    profile: Option<String>,
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
            .wrap_err("failed to generate SSH private key")?;
        Ok(AgentSSHServer {
            agent,
            priv_key: Arc::new(priv_key),
            profile: None,
        })
    }

    pub async fn start(self) -> Result<()> {
        tokio::spawn(self.clone().watch_profiles());
        self.serve(AgentSocketID::SSH).await
    }

    async fn serve(self, id: AgentSocketID) -> Result<()> {
        let listener = match listen(agent_socket_path(id)?, SocketPermMode::Owner).await {
            Ok(l) => l,
            Err(e) => {
                tracing::warn!("failed to listen: {e:?}");
                return Err(e);
            }
        };
        ssh_listen(listener, self).await?;
        Ok(())
    }

    /// Keeps one socket per configured profile, next to the default socket that
    /// follows the active profile.
    async fn watch_profiles(self) {
        let notify = self.agent.cfg.on_reload();
        let mut servers: HashMap<String, JoinHandle<()>> = HashMap::new();
        loop {
            // Register before reading the config so a reload in between isn't missed.
            let notified = notify.notified();
            tokio::pin!(notified);
            notified.as_mut().enable();

            let profiles: Vec<String> = self
                .agent
                .cfg
                .read()
                .await
                .profiles
                .keys()
                .cloned()
                .collect();
            servers.retain(|name, handle| {
                if profiles.contains(name) {
                    return true;
                }
                tracing::debug!(profile = name, "ssh-agent: stopping profile socket");
                // Dropping the listener closes the socket and removes its file.
                handle.abort();
                false
            });
            for name in profiles {
                if servers.contains_key(&name) {
                    continue;
                }
                let server = AgentSSHServer {
                    profile: Some(name.clone()),
                    ..self.clone()
                };
                let id = AgentSocketID::SSHProfile(name.clone());
                let profile = name.clone();
                servers.insert(
                    name,
                    tokio::spawn(async move {
                        if let Err(e) = server.serve(id).await {
                            tracing::warn!(profile, "ssh-agent: profile socket failed: {e:?}");
                        }
                    }),
                );
            }

            notified.await;
        }
    }
}
