use authentik_client::models::DeviceGroupRequest;
use eyre::Result;
use serde::Deserialize;

use crate::util::run;

#[derive(Deserialize, Default)]
struct GroupPlist {
    #[serde(rename = "dsAttrTypeStandard:PrimaryGroupID", default)]
    primary_group_id: Vec<String>,
}

fn list_groupnames() -> Result<Vec<String>> {
    let out = run(std::process::Command::new("dscl").args([".", "list", "/Groups"]))?;
    Ok(out
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('_'))
        .map(str::to_string)
        .collect())
}

fn read_group(name: &str) -> Option<DeviceGroupRequest> {
    let out = run(std::process::Command::new("dscl").args([
        "-plist",
        ".",
        "read",
        &format!("/Groups/{name}"),
        "PrimaryGroupID",
    ]))
    .ok()?;
    let parsed: GroupPlist = plist::from_bytes(out.as_bytes()).ok()?;
    Some(DeviceGroupRequest {
        id: parsed.primary_group_id.into_iter().next()?,
        name: Some(name.to_string()),
    })
}

pub fn gather() -> Result<Vec<DeviceGroupRequest>> {
    Ok(list_groupnames()?
        .into_iter()
        .filter_map(|name| read_group(&name))
        .collect())
}
