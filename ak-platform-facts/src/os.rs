use authentik_client::models::{DeviceFactsOsFamily, OperatingSystemRequest};
use eyre::Result;
use serde::Deserialize;

use crate::query::{non_empty, query_named};

fn family() -> Result<DeviceFactsOsFamily> {
    if cfg!(target_os = "linux") {
        Ok(DeviceFactsOsFamily::Linux)
    } else if cfg!(target_os = "macos") {
        Ok(DeviceFactsOsFamily::MacOs)
    } else if cfg!(target_os = "windows") {
        Ok(DeviceFactsOsFamily::Windows)
    } else {
        crate::util::unsupported_platform("os")
    }
}

#[derive(Deserialize)]
struct OsVersionRow {
    #[serde(default)]
    name: String,
    #[serde(default)]
    version: String,
}

pub fn gather() -> Result<OperatingSystemRequest> {
    let row = query_named::<OsVersionRow>("os_version")?.into_iter().next();
    let (name, version) = match row {
        Some(row) => (non_empty(row.name), non_empty(row.version)),
        None => (None, None),
    };
    Ok(OperatingSystemRequest {
        family: family()?,
        name,
        version,
        arch: Some(std::env::consts::ARCH.to_string()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gather_produces_name_and_version() {
        let os = gather().unwrap();
        assert!(os.name.is_some());
        assert!(os.version.is_some());
    }
}
