use authentik_client::models::DeviceUserRequest;
use eyre::Result;

use crate::query::query_named;

pub fn gather() -> Result<Vec<DeviceUserRequest>> {
    Ok(query_named("users")?
        .into_iter()
        .filter_map(|row| {
            let id = row.get("uid")?.clone();
            let username = row.get("username").filter(|s| !s.is_empty()).cloned();
            let name = row.get("description").filter(|s| !s.is_empty()).cloned();
            let home = row.get("directory").filter(|s| !s.is_empty()).cloned();
            Some(DeviceUserRequest {
                id,
                username,
                name,
                home,
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
