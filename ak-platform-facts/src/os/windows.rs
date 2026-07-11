use authentik_client::models::{DeviceFactsOsFamily, OperatingSystemRequest};
use eyre::Result;
use winreg::RegKey;
use winreg::enums::HKEY_LOCAL_MACHINE;

/// Bails if any of the three registry values are missing rather than
/// falling back to another key.
pub fn gather() -> Result<OperatingSystemRequest> {
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    let key = hklm.open_subkey("SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion")?;

    let product_name: String = key.get_value("ProductName")?;
    let display_version: String = key.get_value("DisplayVersion")?;
    let version: String = key.get_value("LCUVer")?;

    Ok(OperatingSystemRequest {
        family: DeviceFactsOsFamily::Windows,
        name: Some(format!("{product_name} {display_version}").trim().to_string()),
        version: Some(version.trim().to_string()),
        arch: Some(std::env::consts::ARCH.to_string()),
    })
}
