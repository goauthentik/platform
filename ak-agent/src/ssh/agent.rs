use signature::Signer as _;
use ssh_agent_lib::{
    agent::Session,
    error::AgentError,
    proto::{Extension, Identity, PublicCredential, SignRequest},
    ssh_key::Signature,
};

use crate::ssh::{
    ext_ak::EXT_AUTHENTIK_AGENT_TUNNEL,
    ext_session_bind::EXT_OPENSSH_SESSION_BIND,
    txn::SSHAgentTransaction,
};

#[ssh_agent_lib::async_trait]
impl Session for SSHAgentTransaction {
    async fn request_identities(&mut self) -> Result<Vec<Identity>, AgentError> {
        log::trace!("ssh-agent: request_identities()");
        match self.ensure_cert().await {
            Some(cert) => {
                let comment = cert.key_id().to_string();
                Ok(vec![Identity {
                    credential: PublicCredential::Cert(Box::new((*cert).clone())),
                    comment,
                }])
            },
            None => Ok(vec![]),
        }
    }

    async fn sign(&mut self, request: SignRequest) -> Result<Signature, AgentError> {
        log::trace!("ssh-agent: sign()");
        // Attempt cert load (may trigger user authorization prompt).
        // Signing proceeds regardless of cert state, matching Go behavior.
        self.ensure_cert().await;
        self.priv_key
            .try_sign(&request.data)
            .map_err(AgentError::other)
    }

    async fn extension(&mut self, extension: Extension) -> Result<Option<Extension>, AgentError> {
        log::trace!("ssh-agent: extension({})", extension.name);
        match extension.name.as_str() {
            EXT_OPENSSH_SESSION_BIND => self.handle_session_bind(&extension).await,
            EXT_AUTHENTIK_AGENT_TUNNEL => self.handle_agent_tunnel(&extension).await,
            _ => Ok(None),
        }
    }
}
