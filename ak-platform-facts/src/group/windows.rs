use authentik_client::models::DeviceGroupRequest;
use eyre::Result;

use crate::util::run;

pub fn gather() -> Result<Vec<DeviceGroupRequest>> {
    let out = run(std::process::Command::new("powershell").args([
        "-NoProfile",
        "-Command",
        r#"Get-LocalGroup | ForEach-Object { "$($_.Name)|$($_.SID)" }"#,
    ]))?;
    Ok(out
        .lines()
        .filter_map(|line| {
            let (name, sid) = line.trim().split_once('|')?;
            if name.is_empty() || sid.is_empty() {
                return None;
            }
            Some(DeviceGroupRequest {
                id: sid.to_string(),
                name: Some(name.to_string()),
            })
        })
        .collect())
}
