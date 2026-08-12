extern crate pam;

use crate::ENV_SESSION_ID;
use crate::pam_env::pam_get_env;
use crate::session_data::{_delete_session_data, _read_session_data};
use ak_platform::generated::session::session_manager_client::SessionManagerClient;
use ak_platform::generated::session::{CloseSessionRequest, OpenSessionRequest};
use ak_platform::grpc::grpc_request;
use eyre::{Context, Result};
use pam::constants::PamFlag;
use pam::constants::PamResultCode;
use pam::module::PamHandle;
use std::ffi::CStr;

pub fn open_session_impl(
    pamh: &mut PamHandle,
    _args: Vec<&CStr>,
    _flags: PamFlag,
) -> Result<PamResultCode> {
    let sid = match pam_get_env(pamh, ENV_SESSION_ID) {
        Some(t) => t,
        None => {
            tracing::warn!("failed to get session id");
            return Ok(PamResultCode::PAM_IGNORE);
        }
    };
    let sd = _read_session_data(sid.clone()).context("failed to get session data")?;
    _delete_session_data(sid.clone()).context("failed to delete session data")?;

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

pub fn close_session_impl(
    pamh: &mut PamHandle,
    _args: Vec<&CStr>,
    _flags: PamFlag,
) -> Result<PamResultCode> {
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
