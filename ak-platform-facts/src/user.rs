use authentik_client::models::DeviceUserRequest;
use eyre::Result;
use serde::Deserialize;

use crate::query::{non_empty, query_named};

#[derive(Deserialize)]
struct UserRow {
    #[serde(default)]
    uid: String,
    #[serde(default)]
    username: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    directory: String,
}

pub fn gather() -> Result<Vec<DeviceUserRequest>> {
    Ok(query_named::<UserRow>("users")?
        .into_iter()
        .filter_map(|row| {
            if row.uid.is_empty() {
                return None;
            }
            Some(DeviceUserRequest {
                id: row.uid,
                username: non_empty(row.username),
                name: non_empty(row.description),
                home: non_empty(row.directory),
            })
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(not(windows))]
    fn gather_finds_root_user() {
        let users = gather().unwrap_or_default();
        let Some(root) = users.iter().find(|u| u.username.as_deref() == Some("root")) else {
            unreachable!("root user must exist");
        };
        assert_eq!(root.id, "0");
    }
}
