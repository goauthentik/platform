extern crate pam;

use crate::pam_env::pam_get_env;
use crate::session_data::{_delete_session_data, _read_session_data};
use crate::{ENV_SESSION_ID, pam_try_log};
use authentik_sys::grpc::grpc_request;
use authentik_sys::generated::session::session_manager_client::SessionManagerClient;
use authentik_sys::generated::session::{CloseSessionRequest, RegisterSessionRequest};
use pam::constants::PamFlag;
use pam::constants::PamResultCode;
use pam::module::PamHandle;
use std::ffi::CStr;

pub fn open_session_impl(
    pamh: &mut PamHandle,
    _args: Vec<&CStr>,
    _flags: PamFlag,
) -> PamResultCode {
    let sid = match pam_get_env(pamh, ENV_SESSION_ID) {
        Some(t) => t,
        None => {
            log::warn!("failed to get session id");
            return PamResultCode::PAM_IGNORE;
        }
    };
    let sd = pam_try_log!(
        _read_session_data(sid.to_owned()),
        "failed to get session data"
    );
    pam_try_log!(
        _delete_session_data(sid.to_owned()),
        "failed to delete session data"
    );

    let session_info = match grpc_request(async |ch| {
        return Ok(SessionManagerClient::new(ch)
            .register_session(RegisterSessionRequest {
                session_id: sid.clone(),
                username: sd.username.to_owned(),
                token_hash: sd.token.clone(),
                expires_at: sd.expiry,
                pid: std::process::id(),
                ppid: std::os::unix::process::parent_id(),
                local_socket: sd.local_socket.clone(),
            })
            .await?);
    }) {
        Ok(r) => r.into_inner(),
        Err(e) => {
            log::warn!("failed to register session: {e}");
            return PamResultCode::PAM_SESSION_ERR;
        }
    };

    if !session_info.success {
        log::warn!("failed to add session: {}", session_info.error);
        return PamResultCode::PAM_SESSION_ERR;
    }

    PamResultCode::PAM_SUCCESS
}

pub fn close_session_impl(
    pamh: &mut PamHandle,
    _args: Vec<&CStr>,
    _flags: PamFlag,
) -> PamResultCode {
    let sid = match pam_get_env(pamh, ENV_SESSION_ID) {
        Some(t) => t,
        None => {
            log::warn!("failed to get session id");
            return PamResultCode::PAM_IGNORE;
        }
    };

    let session_info = match grpc_request(async |ch| {
        return Ok(SessionManagerClient::new(ch)
            .close_session(CloseSessionRequest {
                session_id: sid.clone(),
                pid: std::process::id(),
            })
            .await?);
    }) {
        Ok(r) => r.into_inner(),
        Err(e) => {
            log::warn!("failed to remove session: {e}");
            return PamResultCode::PAM_SESSION_ERR;
        }
    };

    if !session_info.success {
        log::warn!("failed to remove session");
        return PamResultCode::PAM_SESSION_ERR;
    }

    PamResultCode::PAM_SUCCESS
}
