use ssh_agent_lib::{
    agent::Session,
    error::AgentError,
    proto::{Extension, Identity, ProtoError, SignRequest},
    ssh_key::Signature,
};

use crate::ssh::AgentSSHServer;

#[ssh_agent_lib::async_trait]
impl Session for AgentSSHServer {
    /// Request a list of keys managed by this session.
    async fn request_identities(&mut self) -> Result<Vec<Identity>, AgentError> {
        log::trace!("ssh-agent: request_identities()");
        Err(AgentError::from(ProtoError::UnsupportedCommand {
            command: 11,
        }))
    }

    /// Perform a private key signature operation.
    async fn sign(&mut self, _request: SignRequest) -> Result<Signature, AgentError> {
        log::trace!("ssh-agent: sign()");
        Err(AgentError::from(ProtoError::UnsupportedCommand {
            command: 13,
        }))
    }

    /// Invoke a custom, vendor-specific extension on the agent.
    async fn extension(&mut self, _extension: Extension) -> Result<Option<Extension>, AgentError> {
        log::trace!("ssh-agent: extension({})", _extension.name);
        Err(AgentError::from(ProtoError::UnsupportedCommand {
            command: 27,
        }))
    }
}
