use authentik_client::models::DeviceUserRequest;
use eyre::Result;

fn parse_passwd(content: &str) -> Vec<DeviceUserRequest> {
    content
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                return None;
            }
            let mut fields = line.split(':');
            let username = fields.next()?.to_string();
            let _password = fields.next()?;
            let uid = fields.next()?.to_string();
            let _gid = fields.next()?;
            let gecos = fields.next().unwrap_or_default();
            let home = fields.next().unwrap_or_default();

            let name = gecos
                .split(',')
                .next()
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_string);

            Some(DeviceUserRequest {
                id: uid,
                username: Some(username),
                name,
                home: (!home.is_empty()).then(|| home.to_string()),
            })
        })
        .collect()
}

pub fn gather() -> Result<Vec<DeviceUserRequest>> {
    let content = std::fs::read_to_string("/etc/passwd")?;
    Ok(parse_passwd(&content))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_standard_passwd_entries() {
        let content = "\
root:x:0:0:root:/root:/bin/bash
daemon:x:1:1:daemon:/usr/sbin:/usr/sbin/nologin
jens:x:1000:1000:Jens Langhammer,,,:/home/jens:/bin/bash
# a comment

";
        let users = parse_passwd(content);
        let Some(root) = users.iter().find(|u| u.username.as_deref() == Some("root")) else {
            unreachable!("root user must be parsed");
        };
        assert_eq!(root.id, "0");
        assert_eq!(root.name.as_deref(), Some("root"));
        assert_eq!(root.home.as_deref(), Some("/root"));

        let Some(jens) = users.iter().find(|u| u.username.as_deref() == Some("jens")) else {
            unreachable!("jens user must be parsed");
        };
        assert_eq!(jens.id, "1000");
        assert_eq!(jens.name.as_deref(), Some("Jens Langhammer"));
    }

    #[test]
    fn gather_finds_root_user() {
        let users = gather().unwrap_or_default();
        let Some(root) = users.iter().find(|u| u.username.as_deref() == Some("root")) else {
            unreachable!("root user must exist");
        };
        assert_eq!(root.id, "0");
    }
}
