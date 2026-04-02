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
