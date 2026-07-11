use std::collections::HashMap;

use authentik_client::models::{DeviceFactsOsFamily, OperatingSystemRequest};
use eyre::Result;

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

/// `Some` as long as the file could be opened at all, even if empty —
/// callers fall through to the next candidate file only on open failure,
/// not on missing fields.
fn read_release_file(path: &str) -> Option<HashMap<String, String>> {
    std::fs::read_to_string(path).ok().map(|c| parse_kv(&c))
}

/// `name` is `NAME`, overridden by `DISTRIB_ID` if present; `version` is
/// `VERSION_CODENAME`, overridden in turn by `DISTRIB_RELEASE`,
/// `BUILD_ID`, then `VERSION_ID` (highest priority). Both default to
/// `""` rather than being absent.
fn extract_version(fields: &HashMap<String, String>) -> (String, String) {
    let mut name = fields.get("NAME").cloned().unwrap_or_default();
    if let Some(distrib_id) = fields.get("DISTRIB_ID") {
        name = distrib_id.clone();
    }

    let mut version = fields.get("VERSION_CODENAME").cloned().unwrap_or_default();
    if let Some(v) = fields.get("DISTRIB_RELEASE") {
        version = v.clone();
    }
    if let Some(v) = fields.get("BUILD_ID") {
        version = v.clone();
    }
    if let Some(v) = fields.get("VERSION_ID") {
        version = v.clone();
    }

    (name, version)
}

/// Consulted in order when none of `os-release`/`lsb-release` are
/// present. Each path is paired with the fixed label reported as `name`;
/// the file's first line becomes `version`.
const DISTRO_RELEASE_FILES: &[(&str, &str)] = &[
    ("/etc/redhat-release", "Red Hat"),
    ("/etc/centos-release", "CentOS"),
    ("/etc/fedora-release", "Fedora"),
    ("/etc/debian_version", "Debian"),
    ("/etc/arch-release", "Arch Linux"),
];

fn read_first_line(path: &str) -> String {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|s| s.lines().next().map(str::trim).map(str::to_string))
        .unwrap_or_default()
}

fn distro_release_fallback() -> Option<(String, String)> {
    DISTRO_RELEASE_FILES.iter().find_map(|(path, label)| {
        let content = read_first_line(path);
        (!content.is_empty()).then(|| (label.to_string(), content))
    })
}

/// The third whitespace-separated token of `/proc/version`'s first line,
/// e.g. "Linux version 5.15.0-91-generic ..." -> "5.15.0-91-generic".
fn kernel_version_fallback() -> String {
    let first_line = read_first_line("/proc/version");
    first_line
        .split_whitespace()
        .nth(2)
        .map(str::to_string)
        .unwrap_or_default()
}

fn linux_distribution() -> (String, String) {
    for path in ["/etc/os-release", "/usr/lib/os-release", "/etc/lsb-release"] {
        if let Some(fields) = read_release_file(path) {
            return extract_version(&fields);
        }
    }
    if let Some(found) = distro_release_fallback() {
        return found;
    }
    ("Linux".to_string(), kernel_version_fallback())
}

pub fn gather() -> Result<OperatingSystemRequest> {
    let (name, version) = linux_distribution();
    Ok(OperatingSystemRequest {
        family: DeviceFactsOsFamily::Linux,
        name: Some(name),
        version: Some(version),
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
    fn extract_version_ubuntu_os_release() {
        let f = fields(&[
            ("NAME", "Ubuntu"),
            ("VERSION_ID", "24.04"),
            ("VERSION_CODENAME", "noble"),
        ]);
        assert_eq!(extract_version(&f), ("Ubuntu".to_string(), "24.04".to_string()));
    }

    #[test]
    fn extract_version_ubuntu_lsb_release() {
        let f = fields(&[("DISTRIB_ID", "Ubuntu"), ("DISTRIB_RELEASE", "22.04")]);
        assert_eq!(extract_version(&f), ("Ubuntu".to_string(), "22.04".to_string()));
    }

    #[test]
    fn extract_version_distrib_id_overrides_name() {
        let f = fields(&[("NAME", "Ubuntu"), ("DISTRIB_ID", "Pop!_OS")]);
        assert_eq!(extract_version(&f).0, "Pop!_OS");
    }

    #[test]
    fn extract_version_prefers_version_id() {
        let f = fields(&[
            ("VERSION_ID", "24.04"),
            ("BUILD_ID", "rolling"),
            ("VERSION_CODENAME", "noble"),
        ]);
        assert_eq!(extract_version(&f).1, "24.04");
    }

    #[test]
    fn extract_version_falls_back_to_build_id() {
        let f = fields(&[("BUILD_ID", "rolling"), ("VERSION_CODENAME", "noble")]);
        assert_eq!(extract_version(&f).1, "rolling");
    }

    #[test]
    fn extract_version_falls_back_to_distrib_release() {
        let f = fields(&[("DISTRIB_RELEASE", "22.04"), ("VERSION_CODENAME", "jammy")]);
        assert_eq!(extract_version(&f).1, "22.04");
    }

    #[test]
    fn extract_version_falls_back_to_version_codename() {
        let f = fields(&[("VERSION_CODENAME", "bookworm")]);
        assert_eq!(extract_version(&f).1, "bookworm");
    }

    #[test]
    fn extract_version_empty_when_absent() {
        assert_eq!(extract_version(&fields(&[])), (String::new(), String::new()));
    }

    #[test]
    fn parse_kv_strips_quotes() {
        let f = parse_kv("NAME=\"Ubuntu\"\nVERSION_ID=\"24.04\"\n# comment\n\nFOO=bar");
        assert_eq!(f.get("NAME").map(String::as_str), Some("Ubuntu"));
        assert_eq!(f.get("VERSION_ID").map(String::as_str), Some("24.04"));
        assert_eq!(f.get("FOO").map(String::as_str), Some("bar"));
    }
}
