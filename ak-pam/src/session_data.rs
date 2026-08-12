use serde::{Deserialize, Serialize};

use eyre::{Context, Result};
use std::fs::{File, OpenOptions, remove_file};
use std::io::Write;
use std::os::unix::fs::OpenOptionsExt;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SessionData {
    pub username: String,
    pub local_socket: String,
}

pub fn _session_file(id: String) -> String {
    format!("/tmp/.aksm-{id}")
}

pub fn _read_session_data(id: String) -> Result<SessionData> {
    let path = _session_file(id);
    let file = File::open(path)?;

    let sd: SessionData = serde_json::from_reader(file)?;
    Ok(sd)
}

pub fn _delete_session_data(id: String) -> Result<()> {
    let path = _session_file(id);
    remove_file(path)?;
    Ok(())
}

pub fn _write_session_data(id: String, data: SessionData) -> Result<()> {
    let json_data = serde_json::to_string(&data).context("failed to json encode")?;
    let path = _session_file(id);
    // create_new(true) sets O_EXCL, preventing symlink attacks on the predictable path.
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o400)
        .open(&path)
        .context("failed to create file")?;

    file.write_all(json_data.as_bytes())
        .context("failed to write session data")?;
    Ok(())
}
