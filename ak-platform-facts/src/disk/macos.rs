use eyre::Result;
use serde::Deserialize;

use crate::util::run;

#[derive(Deserialize, Default)]
struct DiskutilPlist {
    #[serde(rename = "Encryption", default)]
    encryption: bool,
    #[serde(rename = "FileVault", default)]
    file_vault: bool,
}

pub fn encryption_enabled(name: &str, _mountpoint: &str) -> Result<bool> {
    let out = run(std::process::Command::new("diskutil").args(["info", "-plist", name]))?;
    let parsed: DiskutilPlist = plist::from_bytes(out.as_bytes())?;
    Ok(parsed.encryption || parsed.file_vault)
}
