use ssh_agent_lib::{
    error::AgentError,
    proto::{Extension, extension::message::SessionBind},
};
use ssh_agent_lib::ssh_encoding::Decode as _;

use crate::ssh::txn::SSHAgentTransaction;

pub const EXT_OPENSSH_SESSION_BIND: &str = "session-bind@openssh.com";

impl SSHAgentTransaction {
    pub async fn handle_session_bind(
        &mut self,
        ext: &Extension,
    ) -> Result<Option<Extension>, AgentError> {
        let raw = ext.details.as_ref();
        let bind = SessionBind::decode(&mut &raw[..]).map_err(AgentError::other)?;

        bind.verify_signature().map_err(AgentError::other)?;

        if bind.session_id.len() > 128 {
            return Err(AgentError::other(std::io::Error::other(
                "session_id exceeds maximum length of 128 bytes",
            )));
        }

        log::debug!(
            "ssh-agent: session-bind: host_key={}, forwarding={}",
            bind.host_key.algorithm(),
            bind.is_forwarding
        );

        self.host_key = Some(bind.host_key);
        self.session_id = Some(bind.session_id);

        Ok(None)
    }
}
