use authentik_client::models::DeviceGroupRequest;
use eyre::Result;
use serde::Deserialize;

use crate::query::{non_empty, query_named};

#[derive(Deserialize)]
struct GroupRow {
    #[serde(default)]
    gid: String,
    #[serde(default)]
    groupname: String,
}

pub fn gather() -> Result<Vec<DeviceGroupRequest>> {
    Ok(query_named::<GroupRow>("groups")?
        .into_iter()
        .filter_map(|row| {
            if row.gid.is_empty() {
                return None;
            }
            Some(DeviceGroupRequest {
                id: row.gid,
                name: non_empty(row.groupname),
            })
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(not(windows))]
    fn gather_finds_gid_zero() {
        // Named "root" on Linux, "wheel" on macOS — gid 0 is the portable
        // invariant, not the name.
        let groups = gather().unwrap_or_default();
        assert!(groups.iter().any(|g| g.id == "0"));
    }
}
