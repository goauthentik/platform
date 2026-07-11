use authentik_client::models::DeviceGroupRequest;
use eyre::Result;

fn parse_group(content: &str) -> Vec<DeviceGroupRequest> {
    content
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                return None;
            }
            let mut fields = line.split(':');
            let name = fields.next()?.to_string();
            let _password = fields.next()?;
            let gid = fields.next()?.to_string();
            Some(DeviceGroupRequest {
                id: gid,
                name: Some(name),
            })
        })
        .collect()
}

pub fn gather() -> Result<Vec<DeviceGroupRequest>> {
    let content = std::fs::read_to_string("/etc/group")?;
    Ok(parse_group(&content))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_standard_group_entries() {
        let content = "\
root:x:0:
daemon:x:1:
staff:x:50:jens,alice
# comment

";
        let groups = parse_group(content);
        let Some(root) = groups.iter().find(|g| g.name.as_deref() == Some("root")) else {
            unreachable!("root group must be parsed");
        };
        assert_eq!(root.id, "0");
        let Some(staff) = groups.iter().find(|g| g.name.as_deref() == Some("staff")) else {
            unreachable!("staff group must be parsed");
        };
        assert_eq!(staff.id, "50");
    }

    #[test]
    fn gather_finds_root_group() {
        let groups = gather().unwrap_or_default();
        let Some(root) = groups.iter().find(|g| g.name.as_deref() == Some("root")) else {
            unreachable!("root group must exist");
        };
        assert_eq!(root.id, "0");
    }
}
