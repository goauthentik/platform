use pam::constants::PamResultCode;

use serde::{Deserialize, Serialize};

use std::fs::{File, Permissions, remove_file};
use std::io::{Read, Write};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct SessionData {
    pub username: String,
    pub token: String,
    pub local_socket: String,
}

pub fn session_file_path_for(base_dir: impl AsRef<Path>, id: &str) -> PathBuf {
    base_dir.as_ref().join(format!(".aksm-{id}"))
}

pub fn _session_file(id: String) -> String {
    session_file_path_for("/tmp", &id)
        .to_string_lossy()
        .into_owned()
}

fn serialize_session_data(data: &SessionData) -> Result<String, PamResultCode> {
    serde_json::to_string(data).map_err(|e| {
        log::warn!("failed to json encode: {e}");
        PamResultCode::PAM_SESSION_ERR
    })
}

fn read_session_data(reader: impl Read) -> Result<SessionData, PamResultCode> {
    serde_json::from_reader(reader).map_err(|e| {
        log::warn!("failed to decode session data: {e}");
        PamResultCode::PAM_AUTH_ERR
    })
}

fn write_session_data_file(path: &Path, data: &SessionData) -> Result<(), PamResultCode> {
    let json_data = serialize_session_data(data)?;
    let mut file = match File::create(path) {
        Ok(f) => f,
        Err(e) => {
            log::warn!("failed to create file: {e}");
            return Err(PamResultCode::PAM_SESSION_ERR);
        }
    };

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

pub fn _read_session_data(id: String) -> Result<SessionData, PamResultCode> {
    let path = session_file_path_for("/tmp", &id);
    let file = match File::open(path) {
        Ok(f) => f,
        Err(e) => {
            log::warn!("failed to open file: {e}");
            return Err(PamResultCode::PAM_SESSION_ERR);
        }
    };

    read_session_data(file)
}

pub fn _delete_session_data(id: String) -> Result<(), PamResultCode> {
    let path = session_file_path_for("/tmp", &id);
    match remove_file(path) {
        Ok(_) => Ok(()),
        Err(e) => {
            log::warn!("Failed to remove session data: {e}");
            Err(PamResultCode::PAM_SESSION_ERR)
        }
    }
}

pub fn _write_session_data(id: String, data: SessionData) -> Result<(), PamResultCode> {
    let path = session_file_path_for("/tmp", &id);
    write_session_data_file(&path, &data)
}

#[cfg(test)]
#[path = "session_data_tests.rs"]
mod tests;
