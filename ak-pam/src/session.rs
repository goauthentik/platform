extern crate pam;

use crate::pam_env::pam_get_env;
use crate::{ENV_SESSION_ID, username};
use ak_platform::generated::session::session_manager_client::SessionManagerClient;
use ak_platform::generated::session::{CloseSessionRequest, CreateSessionRequest};
use ak_platform::grpc::grpc_request;
use eyre::{Context, ContextCompat, Result};
use pam::constants::PamFlag;
use pam::constants::PamResultCode;
use pam::module::PamHandle;
use std::ffi::CStr;

pub const SSH_AUTH_INFO_0: &str = "SSH_AUTH_INFO_0";

pub fn open_session_impl(pamh: &mut PamHandle) -> Result<PamResultCode> {
    let username = username(pamh)?;
    let ssh_auth_info = pam_get_env(pamh, SSH_AUTH_INFO_0).context("failed to get auth info")?;

    let session_info = grpc_request(async |ch| {
        return Ok(SessionManagerClient::new(ch)
            .create_session(CreateSessionRequest {
                username: username.clone(),
                token: None,
                ssh_auth: Some(ssh_auth_info.clone()),
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

pub fn close_session_impl(pamh: &mut PamHandle) -> Result<PamResultCode> {
    let sid = match pam_get_env(pamh, ENV_SESSION_ID) {
        Some(t) => t,
        None => {
            tracing::warn!("failed to get session id");
            return Ok(PamResultCode::PAM_IGNORE);
        }
    };

    let session_info = grpc_request(async |ch| {
        return Ok(SessionManagerClient::new(ch)
            .close_session(CloseSessionRequest {
                session_id: sid.clone(),
                pid: std::process::id(),
            })
            .await?);
    })
    .context("failed to remove session: {e}")?
    .into_inner();

    if !session_info.success {
        tracing::warn!("failed to remove session");
        return Ok(PamResultCode::PAM_SESSION_ERR);
    }

    Ok(PamResultCode::PAM_SUCCESS)
}
