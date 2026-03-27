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
mod tests {
    use super::{
        SessionData, read_session_data, serialize_session_data, session_file_path_for,
        write_session_data_file,
    };
    use pam::constants::PamResultCode;
    use std::fs;
    use std::io::Cursor;
    use std::os::unix::fs::PermissionsExt;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn sample_session_data() -> SessionData {
        SessionData {
            username: "alice".to_owned(),
            token: "token".to_owned(),
            local_socket: "/tmp/socket".to_owned(),
        }
    }

    #[test]
    fn builds_session_paths_in_the_requested_directory() {
        assert_eq!(
            session_file_path_for("/var/tmp", "abc123"),
            PathBuf::from("/var/tmp/.aksm-abc123")
        );
    }

    #[test]
    fn round_trips_session_data_through_json() {
        let data = sample_session_data();
        let encoded = serialize_session_data(&data).expect("should serialize");

        let decoded = read_session_data(Cursor::new(encoded)).expect("should deserialize");

        assert_eq!(decoded.username, data.username);
        assert_eq!(decoded.token, data.token);
        assert_eq!(decoded.local_socket, data.local_socket);
    }

    #[test]
    fn rejects_invalid_session_json() {
        assert_eq!(
            read_session_data(Cursor::new("{not-json")),
            Err(PamResultCode::PAM_AUTH_ERR)
        );
    }

    #[test]
    fn writes_session_data_with_read_only_permissions() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock should be after unix epoch")
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("authentik-pam-test-{unique}"));
        fs::create_dir(&dir).expect("should create temp dir");
        let path = dir.join(".aksm-session");

        write_session_data_file(&path, &sample_session_data()).expect("should write session file");

        let metadata = fs::metadata(&path).expect("should stat session file");
        assert_eq!(metadata.permissions().mode() & 0o777, 0o400);

        fs::remove_file(&path).expect("should clean up session file");
        fs::remove_dir(&dir).expect("should clean up temp dir");
    }
}
