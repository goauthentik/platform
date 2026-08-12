use ak_platform::{
    generated::session::{CreateSessionRequest, session_manager_client::SessionManagerClient},
    grpc::grpc_request,
};
use eyre::{Context, Result};
use pam::constants::PamResultCode;

pub const SSH_AUTH_INFO_0: &str = "SSH_AUTH_INFO_0";

pub fn open_session_ssh(username: String, ssh_auth: String) -> Result<PamResultCode> {
    let ssh_cert = ssh_auth.strip_prefix("publickey ").unwrap_or(&ssh_auth);
    let session_info = grpc_request(async |ch| {
        return Ok(SessionManagerClient::new(ch)
            .create_session(CreateSessionRequest {
                username: username.clone(),
                token: None,
                ssh_auth: Some(ssh_cert.to_owned()),
                pid: std::process::id(),
                ppid: std::os::unix::process::parent_id(),
            })
            .await?);
    })
    .context("failed to create session")?
    .into_inner();

    if !session_info.success {
        tracing::warn!("failed to add session");
        return Ok(PamResultCode::PAM_SESSION_ERR);
    }

    Ok(PamResultCode::PAM_SUCCESS)
}
