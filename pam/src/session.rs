extern crate pam;

use crate::config::Config;
use crate::generated::create_grpc_client;
use crate::generated::pam_session::{CloseSessionRequest, RegisterSessionRequest};
use crate::pam_env::pam_get_env;
use crate::{DATA_CLIENT, ENV_SESSION_ID};
use pam::constants::PamResultCode;
use pam::module::PamHandle;
use pam::{constants::PamFlag, pam_try};
use rand::Rng;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::ffi::CStr;
use std::fs::File;
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

    let sid = pam_get_env(pamh, ENV_SESSION_ID).unwrap();
    let mut sd = pam_try!(_read_session_data(sid.to_owned()));

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
            log::warn!("Failed to create runtime: {}", e);
            return PamResultCode::PAM_SESSION_ERR;
        }
    };
    let mut client = match rt.block_on(create_grpc_client(config)) {
        Ok(res) => res,
        Err(e) => {
            log::warn!("Failed to create grpc client: {}", e);
            return PamResultCode::PAM_SESSION_ERR;
        }
    };
    let response = match rt.block_on(client.register_session(request)) {
        Ok(res) => res,
        Err(e) => {
            log::warn!("failed to send GRPC request: {}", e);
            return PamResultCode::PAM_SESSION_ERR;
        }
    };
    let session_info = response.into_inner();

    if !session_info.success {
        log::warn!("failed to add session: {}", session_info.error.to_string());
        return PamResultCode::PAM_SESSION_ERR;
    }

    pam_try!(pamh.set_data(DATA_CLIENT, Box::new(client)));
    PamResultCode::PAM_SUCCESS
}

pub fn close_session_impl(
    pamh: &mut PamHandle,
    _args: Vec<&CStr>,
    _flags: PamFlag,
) -> PamResultCode {
    let config = Config::from_file("/etc/authentik/host.yaml").expect("Failed to load config");

    let id = pam_get_env(pamh, ENV_SESSION_ID).unwrap();
    let request = tonic::Request::new(CloseSessionRequest {
        session_id: id,
        pid: std::process::id(),
    });

    let rt = match Runtime::new() {
        Ok(rt) => rt,
        Err(e) => {
            log::warn!("Failed to create runtime: {}", e);
            return PamResultCode::PAM_SESSION_ERR;
        }
    };
    let mut client = match rt.block_on(create_grpc_client(config)) {
        Ok(res) => res,
        Err(e) => {
            log::warn!("Failed to create grpc client: {}", e);
            return PamResultCode::PAM_SESSION_ERR;
        }
    };
    let response = match rt.block_on(client.close_session(request)) {
        Ok(res) => res,
        Err(e) => {
            log::warn!("failed to send GRPC request: {}", e);
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

pub fn _read_session_data(id: String) -> Result<SessionData, PamResultCode> {
    let path = format!("/tmp/.aksm-{}", id);
    let file = File::open(path).expect("Could not create file!");

    return match serde_json::from_reader(file) {
        Ok(t) => Ok(t),
        Err(e) => {
            log::warn!("failed to write session data: {}", e);
            return Err(PamResultCode::PAM_AUTH_ERR);
        }
    };
}

pub fn _write_session_data(id: String, data: SessionData) -> Result<(), PamResultCode> {
    let json_data = serde_json::to_string(&data).unwrap();
    let path = format!("/tmp/.aksm-{}", id);
    let mut file = File::create(path).expect("Could not create file!");

    let mut permissions = file.metadata().unwrap().permissions();
    permissions.set_mode(0o400);
    file.set_permissions(permissions).unwrap();

    return match file.write_all(json_data.as_bytes()) {
        Ok(_) => Ok(()),
        Err(e) => {
            log::warn!("failed to write session data: {}", e);
            return Err(PamResultCode::PAM_AUTH_ERR);
        }
    };
}

pub fn _generate_id() -> String {
    const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ\
                            abcdefghijklmnopqrstuvwxyz\
                            0123456789";
    const PASSWORD_LEN: usize = 30;
    let mut rng = rand::rng();

    return (0..PASSWORD_LEN)
        .map(|_| {
            let idx = rng.random_range(0..CHARSET.len());
            CHARSET[idx] as char
        })
        .collect();
}

pub fn hash_token(token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    hex::encode(hasher.finalize())
}
