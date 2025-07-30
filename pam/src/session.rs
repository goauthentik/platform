extern crate pam;

use crate::config::Config;
use crate::generated::create_grpc_client;
use crate::generated::pam_session::{CloseSessionRequest, RegisterSessionRequest};
use crate::pam_env::pam_get_env;
use crate::{DATA_CLIENT, ENV_SESSION_ID, pam_try_log};
use pam::constants::PamFlag;
use pam::constants::PamResultCode;
use pam::module::PamHandle;
use rand::Rng;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::ffi::CStr;
use std::fs::{File, Permissions, remove_file};
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use tokio::runtime::Runtime;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SessionData {
    pub username: String,
    pub token: String,
    pub expiry: i64,
    pub local_socket: String,
}

pub fn open_session_impl(
    pamh: &mut PamHandle,
    _args: Vec<&CStr>,
    _flags: PamFlag,
) -> PamResultCode {
    let config = Config::from_file("/etc/authentik/host.yaml").expect("Failed to load config");

    let sid = match pam_get_env(pamh, ENV_SESSION_ID) {
        Some(t) => t,
        None => {
            log::warn!("failed to get session id");
            return PamResultCode::PAM_IGNORE;
        }
    };
    let mut sd = pam_try_log!(
        _read_session_data(sid.to_owned()),
        "failed to get session data"
    );
    pam_try_log!(
        _delete_session_data(sid.to_owned()),
        "failed to delete session data"
    );

    if !config.pam.terminate_on_expiry {
        sd.expiry = -1;
    }

    let request = tonic::Request::new(RegisterSessionRequest {
        session_id: sid,
        username: sd.username.to_owned(),
        token_hash: sd.token,
        expires_at: sd.expiry,
        pid: std::process::id(),
        ppid: std::os::unix::process::parent_id(),
        local_socket: sd.local_socket,
    });

    let rt = match Runtime::new() {
        Ok(rt) => rt,
        Err(e) => {
            log::warn!("Failed to create runtime: {e}");
            return PamResultCode::PAM_SESSION_ERR;
        }
    };
    let mut client = match rt.block_on(create_grpc_client(config)) {
        Ok(res) => res,
        Err(e) => {
            log::warn!("Failed to create grpc client: {e}");
            return PamResultCode::PAM_SESSION_ERR;
        }
    };
    let response = match rt.block_on(client.register_session(request)) {
        Ok(res) => res,
        Err(e) => {
            log::warn!("failed to send GRPC request: {e}");
            return PamResultCode::PAM_SESSION_ERR;
        }
    };
    let session_info = response.into_inner();

    if !session_info.success {
        log::warn!("failed to add session: {}", session_info.error);
        return PamResultCode::PAM_SESSION_ERR;
    }

    pam_try_log!(
        pamh.set_data(DATA_CLIENT, Box::new(client)),
        "failed to set client data"
    );
    PamResultCode::PAM_SUCCESS
}

pub fn close_session_impl(
    pamh: &mut PamHandle,
    _args: Vec<&CStr>,
    _flags: PamFlag,
) -> PamResultCode {
    let config = Config::from_file("/etc/authentik/host.yaml").expect("Failed to load config");

    let sid = match pam_get_env(pamh, ENV_SESSION_ID) {
        Some(t) => t,
        None => {
            log::warn!("failed to get session id");
            return PamResultCode::PAM_IGNORE;
        }
    };
    let request = tonic::Request::new(CloseSessionRequest {
        session_id: sid,
        pid: std::process::id(),
    });

    let rt = match Runtime::new() {
        Ok(rt) => rt,
        Err(e) => {
            log::warn!("Failed to create runtime: {e}");
            return PamResultCode::PAM_SESSION_ERR;
        }
    };
    let mut client = match rt.block_on(create_grpc_client(config)) {
        Ok(res) => res,
        Err(e) => {
            log::warn!("Failed to create grpc client: {e}");
            return PamResultCode::PAM_SESSION_ERR;
        }
    };
    let response = match rt.block_on(client.close_session(request)) {
        Ok(res) => res,
        Err(e) => {
            log::warn!("failed to send GRPC request: {e}");
            return PamResultCode::PAM_SESSION_ERR;
        }
    };
    let session_info = response.into_inner();

    if !session_info.success {
        log::warn!("failed to remove session");
        return PamResultCode::PAM_SESSION_ERR;
    }

    PamResultCode::PAM_SUCCESS
}

pub fn _session_file(id: String) -> String {
    format!("/tmp/.aksm-{id}")
}

pub fn _read_session_data(id: String) -> Result<SessionData, PamResultCode> {
    let path = _session_file(id);
    let file = File::open(path).expect("Could not create file!");

    match serde_json::from_reader(file) {
        Ok(t) => Ok(t),
        Err(e) => {
            log::warn!("failed to write session data: {e}");
            Err(PamResultCode::PAM_AUTH_ERR)
        }
    }
}

pub fn _delete_session_data(id: String) -> Result<(), PamResultCode> {
    let path = _session_file(id);
    match remove_file(path) {
        Ok(_) => Ok(()),
        Err(e) => {
            log::warn!("Failed to remove session data: {e}");
            Err(PamResultCode::PAM_SESSION_ERR)
        }
    }
}

pub fn _write_session_data(id: String, data: SessionData) -> Result<(), PamResultCode> {
    let json_data = match serde_json::to_string(&data) {
        Ok(j) => j,
        Err(e) => {
            log::warn!("failed to json encode: {e}");
            return Err(PamResultCode::PAM_SESSION_ERR);
        }
    };
    let path = _session_file(id);
    let mut file = File::create(path).expect("Could not create file!");

    match file.set_permissions(Permissions::from_mode(0o400)) {
        Ok(_) => {}
        Err(e) => {
            log::warn!("failed to get file permissions: {e}");
            return Err(PamResultCode::PAM_SESSION_ERR);
        }
    };

    match file.write_all(json_data.as_bytes()) {
        Ok(_) => Ok(()),
        Err(e) => {
            log::warn!("failed to write session data: {e}");
            Err(PamResultCode::PAM_AUTH_ERR)
        }
    }
}

pub fn _generate_id() -> String {
    const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ\
                            abcdefghijklmnopqrstuvwxyz\
                            0123456789";
    const PASSWORD_LEN: usize = 30;
    let mut rng = rand::rng();

    (0..PASSWORD_LEN)
        .map(|_| {
            let idx = rng.random_range(0..CHARSET.len());
            CHARSET[idx] as char
        })
        .collect()
}

pub fn hash_token(token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    hex::encode(hasher.finalize())
}
