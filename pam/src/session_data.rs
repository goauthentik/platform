use pam::constants::PamResultCode;

use rand::Rng;
use serde::{Deserialize, Serialize};

use sha2::{Digest, Sha256};

use std::fs::{File, Permissions, remove_file};
use std::io::Write;
use std::os::unix::fs::PermissionsExt;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SessionData {
    pub username: String,
    pub token: String,
    pub expiry: i64,
    pub local_socket: String,
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

pub fn hash_token(token: String) -> String {
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    hex::encode(hasher.finalize())
}
