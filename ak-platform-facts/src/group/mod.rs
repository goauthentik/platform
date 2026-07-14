use authentik_client::models::DeviceGroupRequest;
use eyre::Result;

use crate::query::query_named;

pub fn gather() -> Result<Vec<DeviceGroupRequest>> {
    Ok(query_named("groups")?
        .into_iter()
        .filter_map(|row| {
            let id = row.get("gid")?.clone();
            let name = row.get("groupname").filter(|s| !s.is_empty()).cloned();
            Some(DeviceGroupRequest { id, name })
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
