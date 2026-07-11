use authentik_client::models::{DeviceFactsOsFamily, OperatingSystemRequest};
use eyre::Result;

use crate::util::run;

fn sw_vers(arg: &str) -> Option<String> {
    run(std::process::Command::new("sw_vers").arg(arg))
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

pub fn gather() -> Result<OperatingSystemRequest> {
    Ok(OperatingSystemRequest {
        family: DeviceFactsOsFamily::MacOs,
        name: sw_vers("-productName"),
        version: sw_vers("-productVersion"),
        arch: Some(std::env::consts::ARCH.to_string()),
    })
}
