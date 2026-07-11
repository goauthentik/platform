use std::collections::HashMap;

use authentik_client::models::{DeviceFactsOsFamily, OperatingSystemRequest};
use eyre::Result;

/// Parses `KEY=VALUE` files such as `/etc/os-release`, stripping surrounding
/// quotes from values.
fn parse_kv(content: &str) -> HashMap<String, String> {
    content
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                return None;
            }
            let (key, value) = line.split_once('=')?;
            let value = value.trim().trim_matches('"').trim_matches('\'');
            Some((key.trim().to_string(), value.to_string()))
        })
        .collect()
}

fn read_release_file(path: &str) -> Option<HashMap<String, String>> {
    std::fs::read_to_string(path).ok().map(|c| parse_kv(&c))
}

/// Ported from Go's `extractVersion`: prefers `VERSION_ID`, falling back
/// through `BUILD_ID`, `DISTRIB_RELEASE`, and finally `VERSION_CODENAME`.
fn extract_version(fields: &HashMap<String, String>) -> Option<String> {
    fields
        .get("VERSION_ID")
        .or_else(|| fields.get("BUILD_ID"))
        .or_else(|| fields.get("DISTRIB_RELEASE"))
        .or_else(|| fields.get("VERSION_CODENAME"))
        .cloned()
}

/// Distro-specific release files consulted (in order) when none of
/// `os-release`/`lsb-release` are present.
const DISTRO_RELEASE_FILES: &[&str] = &[
    "/etc/redhat-release",
    "/etc/centos-release",
    "/etc/fedora-release",
    "/etc/debian_version",
    "/etc/arch-release",
];

fn distro_release_fallback() -> Option<String> {
    DISTRO_RELEASE_FILES.iter().find_map(|path| {
        std::fs::read_to_string(path)
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
    })
}

/// Last-resort version fallback: parses the kernel version out of
/// `/proc/version` (e.g. "Linux version 5.15.0-91-generic ...").
fn kernel_version_fallback() -> Option<String> {
    let content = std::fs::read_to_string("/proc/version").ok()?;
    let mut words = content.split_whitespace();
    while let Some(word) = words.next() {
        if word == "version" {
            return words.next().map(|s| s.to_string());
        }
    }
    None
}

pub fn gather() -> Result<OperatingSystemRequest> {
    let fields = read_release_file("/etc/os-release")
        .or_else(|| read_release_file("/usr/lib/os-release"))
        .or_else(|| read_release_file("/etc/lsb-release"));

    let (name, version) = match &fields {
        Some(fields) => (
            fields
                .get("PRETTY_NAME")
                .or_else(|| fields.get("NAME"))
                .cloned(),
            extract_version(fields),
        ),
        None => (distro_release_fallback(), None),
    };

    let version = version.or_else(kernel_version_fallback);

    Ok(OperatingSystemRequest {
        family: DeviceFactsOsFamily::Linux,
        name,
        version,
        arch: Some(std::env::consts::ARCH.to_string()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fields(pairs: &[(&str, &str)]) -> HashMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    #[test]
    fn extract_version_prefers_version_id() {
        let f = fields(&[
            ("VERSION_ID", "24.04"),
            ("BUILD_ID", "rolling"),
            ("VERSION_CODENAME", "noble"),
        ]);
        assert_eq!(extract_version(&f).as_deref(), Some("24.04"));
    }

    #[test]
    fn extract_version_falls_back_to_build_id() {
        let f = fields(&[("BUILD_ID", "rolling"), ("VERSION_CODENAME", "noble")]);
        assert_eq!(extract_version(&f).as_deref(), Some("rolling"));
    }

    #[test]
    fn extract_version_falls_back_to_distrib_release() {
        let f = fields(&[("DISTRIB_RELEASE", "22.04"), ("VERSION_CODENAME", "jammy")]);
        assert_eq!(extract_version(&f).as_deref(), Some("22.04"));
    }

    #[test]
    fn extract_version_falls_back_to_version_codename() {
        let f = fields(&[("VERSION_CODENAME", "bookworm")]);
        assert_eq!(extract_version(&f).as_deref(), Some("bookworm"));
    }

    #[test]
    fn extract_version_none_when_absent() {
        assert_eq!(extract_version(&fields(&[])), None);
    }

    #[test]
    fn parse_kv_strips_quotes() {
        let f = parse_kv("NAME=\"Ubuntu\"\nVERSION_ID=\"24.04\"\n# comment\n\nFOO=bar");
        assert_eq!(f.get("NAME").map(String::as_str), Some("Ubuntu"));
        assert_eq!(f.get("VERSION_ID").map(String::as_str), Some("24.04"));
        assert_eq!(f.get("FOO").map(String::as_str), Some("bar"));
    }
}
