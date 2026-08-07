#[cfg(target_os = "windows")]
mod windows;

use std::collections::HashMap;

use serde_json::Value;

#[cfg(windows)]
fn ssh_host_key_dir() -> &'static str {
    "C:\\ProgramData\\ssh"
}

#[cfg(not(windows))]
fn ssh_host_key_dir() -> &'static str {
    "/etc/ssh"
}

/// Reads local SSH host public keys directly rather than scanning over the
/// network — doesn't depend on sshd already listening.
fn ssh_host_keys() -> Vec<String> {
    ssh_host_keys_from(ssh_host_key_dir())
}

fn ssh_host_keys_from(dir: &str) -> Vec<String> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut keys: Vec<String> = entries
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.starts_with("ssh_host_") && n.ends_with("_key.pub"))
        })
        .filter_map(|p| std::fs::read_to_string(p).ok())
        // Store as `localhost <type> <key>` (drop the `.pub` file's trailing
        // comment). authentik's device lookup matches this string exactly against
        // the comment-less key the agent sends, mirroring Go's `ssh-keyscan` output.
        .filter_map(|s| {
            let mut parts = s.split_whitespace();
            let typ = parts.next()?;
            let key = parts.next()?;
            Some(format!("localhost {typ} {key}"))
        })
        .collect();
    keys.sort();
    keys
}

#[cfg(target_os = "windows")]
fn rdp_cert_fingerprint() -> String {
    windows::rdp_cert_fingerprint().unwrap_or_default()
}

#[cfg(not(target_os = "windows"))]
fn rdp_cert_fingerprint() -> String {
    String::new()
}

pub fn gather() -> HashMap<String, Value> {
    let mut vendor = HashMap::new();
    vendor.insert(
        "agent_version".to_string(),
        Value::String(ak_meta::full_version()),
    );
    vendor.insert("ssh_host_keys".to_string(), Value::from(ssh_host_keys()));
    vendor.insert(
        "rdp_cert_fingerprint".to_string(),
        Value::String(rdp_cert_fingerprint()),
    );
    vendor
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ssh_valid() {
        let keys = ssh_host_keys();
        for key in keys {
            assert!(key.starts_with("localhost "));
        }
    }

    #[test]
    fn ssh_host_keys_strips_comment_and_filters() {
        let dir = tempfile::tempdir().unwrap();
        // `.pub` files carry a trailing comment (e.g. `root@host`) that must be
        // dropped so the fact matches authentik's comment-less device lookup.
        std::fs::write(
            dir.path().join("ssh_host_ed25519_key.pub"),
            "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAExampleKey root@myhost\n",
        )
        .unwrap();
        std::fs::write(
            dir.path().join("ssh_host_rsa_key.pub"),
            "ssh-rsa AAAAB3NzaC1ycExampleRsaKey some comment here\n",
        )
        .unwrap();
        // Non-host-key files and private keys must be ignored.
        std::fs::write(dir.path().join("ssh_host_ed25519_key"), "PRIVATE\n").unwrap();
        std::fs::write(dir.path().join("moduli"), "irrelevant\n").unwrap();

        let keys = ssh_host_keys_from(dir.path().to_str().unwrap());

        // Sorted, `localhost <type> <key>`, comment(s) stripped.
        assert_eq!(
            keys,
            vec![
                "localhost ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAExampleKey".to_string(),
                "localhost ssh-rsa AAAAB3NzaC1ycExampleRsaKey".to_string(),
            ]
        );
    }

    #[test]
    fn ssh_host_keys_missing_dir_is_empty() {
        assert!(ssh_host_keys_from("/nonexistent/ssh/dir/xyz").is_empty());
    }
}
