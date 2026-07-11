use eyre::Result;
use serde::Deserialize;
use wmi::{COMLibrary, WMIConnection};

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Win32EncryptableVolume {
    drive_letter: Option<String>,
    encryption_method: Option<u32>,
    protection_status: Option<u32>,
}

pub fn encryption_enabled(_name: &str, mountpoint: &str) -> Result<bool> {
    let con = WMIConnection::with_namespace_path(
        "ROOT\\CIMV2\\Security\\MicrosoftVolumeEncryption",
        COMLibrary::new()?,
    )?;
    let volumes: Vec<Win32EncryptableVolume> = con.query()?;
    let mountpoint = mountpoint.trim_end_matches('\\');
    Ok(volumes.iter().any(|v| {
        v.drive_letter
            .as_deref()
            .is_some_and(|dl| dl.eq_ignore_ascii_case(mountpoint))
            && v.encryption_method.unwrap_or(0) != 0
            && v.protection_status == Some(1)
    }))
}
