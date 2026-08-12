extern crate pam;

use crate::pam_env::pam_get_env;
use crate::session::session_data::SessionData;
use crate::session::ssh::{SSH_AUTH_INFO_0, open_session_ssh};
use crate::{ENV_SESSION_ID, username};
use ak_platform::generated::session::session_manager_client::SessionManagerClient;
use ak_platform::generated::session::{CloseSessionRequest, OpenSessionRequest};
use ak_platform::grpc::grpc_request;
use eyre::{Context, Result};
use pam::constants::PamResultCode;
use pam::module::PamHandle;

pub mod session_data;
pub mod ssh;

pub fn open_session_impl(pamh: &mut PamHandle) -> Result<PamResultCode> {
    let username = username(pamh)?;
    if let Some(ssh_auth_info) = pam_get_env(pamh, SSH_AUTH_INFO_0) {
        return open_session_ssh(pamh, username, ssh_auth_info);
    }

    let sid = match pam_get_env(pamh, ENV_SESSION_ID) {
        Some(t) => t,
        None => {
            tracing::warn!("failed to get session id");
            return Ok(PamResultCode::PAM_IGNORE);
        }
    };
    let sd = SessionData::read(sid.clone()).context("failed to get session data")?;
    SessionData::delete(sid.clone()).context("failed to delete session data")?;

    let session_info = grpc_request(async |ch| {
        return Ok(SessionManagerClient::new(ch)
            .open_session(OpenSessionRequest {
                session_id: sid.clone(),
                pid: std::process::id(),
                ppid: std::os::unix::process::parent_id(),
                local_socket: sd.local_socket.clone(),
            })
            .await?);
    })
    .context("failed to register session")?
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
