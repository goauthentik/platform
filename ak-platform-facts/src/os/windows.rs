use authentik_client::models::{DeviceFactsOsFamily, OperatingSystemRequest};
use eyre::Result;
use winreg::RegKey;
use winreg::enums::HKEY_LOCAL_MACHINE;

pub fn gather() -> Result<OperatingSystemRequest> {
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    let key = hklm.open_subkey("SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion")?;

    let name: Option<String> = key.get_value("ProductName").ok();
    let version: Option<String> = key
        .get_value("DisplayVersion")
        .ok()
        .or_else(|| key.get_value("LCUVer").ok())
        .or_else(|| key.get_value("CurrentBuildNumber").ok());

    Ok(OperatingSystemRequest {
        family: DeviceFactsOsFamily::Windows,
        name,
        version,
        arch: Some(std::env::consts::ARCH.to_string()),
    })
}
