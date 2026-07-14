#[cfg(target_os = "windows")]
mod windows;

use std::{collections::HashMap, ops::Add};

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
    let Ok(entries) = std::fs::read_dir(ssh_host_key_dir()) else {
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
        .map(|s| "localhost ".to_string().add(&s).trim().to_string())
        .filter(|s| !s.is_empty())
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
}
