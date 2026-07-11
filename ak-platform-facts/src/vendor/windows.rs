use eyre::Result;
use serde::Deserialize;
use wmi::{COMLibrary, WMIConnection};

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Win32TsGeneralSetting {
    ssl_certificate_sha1_hash: Option<String>,
}

pub fn rdp_cert_fingerprint() -> Result<String> {
    let con = WMIConnection::with_namespace_path("ROOT\\CIMV2\\TerminalServices", COMLibrary::new()?)?;
    let settings: Vec<Win32TsGeneralSetting> = con.query()?;
    Ok(settings
        .into_iter()
        .find_map(|s| s.ssl_certificate_sha1_hash)
        .unwrap_or_default())
}
