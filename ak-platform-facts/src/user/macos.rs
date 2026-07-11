use authentik_client::models::DeviceUserRequest;
use eyre::Result;
use serde::Deserialize;

use crate::util::run;

#[derive(Deserialize, Default)]
struct UserPlist {
    #[serde(rename = "dsAttrTypeStandard:UniqueID", default)]
    unique_id: Vec<String>,
    #[serde(rename = "dsAttrTypeStandard:RealName", default)]
    real_name: Vec<String>,
    #[serde(rename = "dsAttrTypeStandard:NFSHomeDirectory", default)]
    home_directory: Vec<String>,
}

fn list_usernames() -> Result<Vec<String>> {
    let out = run(std::process::Command::new("dscl").args([".", "list", "/Users"]))?;
    Ok(out.lines().map(str::trim).filter(|l| !l.is_empty()).map(str::to_string).collect())
}

fn read_user(username: &str) -> Option<DeviceUserRequest> {
    let out = run(std::process::Command::new("dscl").args([
        "-plist",
        ".",
        "read",
        &format!("/Users/{username}"),
    ]))
    .ok()?;
    let parsed: UserPlist = plist::from_bytes(out.as_bytes()).ok()?;

    Some(DeviceUserRequest {
        id: parsed.unique_id.into_iter().next()?,
        username: Some(username.to_string()),
        name: parsed.real_name.into_iter().next(),
        home: parsed.home_directory.into_iter().next(),
    })
}

pub fn gather() -> Result<Vec<DeviceUserRequest>> {
    Ok(list_usernames()?
        .into_iter()
        .filter_map(|username| read_user(&username))
        .collect())
}
