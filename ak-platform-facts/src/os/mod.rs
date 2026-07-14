use authentik_client::models::{DeviceFactsOsFamily, OperatingSystemRequest};
use eyre::Result;

use crate::query::query_named;

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

pub fn gather() -> Result<OperatingSystemRequest> {
    let row = query_named("os_version")?.into_iter().next();
    Ok(OperatingSystemRequest {
        family: family()?,
        name: row
            .as_ref()
            .and_then(|r| r.get("name"))
            .filter(|s| !s.is_empty())
            .cloned(),
        version: row
            .as_ref()
            .and_then(|r| r.get("version"))
            .filter(|s| !s.is_empty())
            .cloned(),
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
