use authentik_client::models::DeviceUserRequest;
use eyre::Result;
use serde::Deserialize;

use crate::util::run;

#[derive(Deserialize)]
struct Sid {
    #[serde(rename = "Value")]
    value: String,
}

#[derive(Deserialize)]
struct RawUser {
    #[serde(rename = "Name")]
    name: String,
    #[serde(rename = "SID")]
    sid: Sid,
    #[serde(rename = "FullName")]
    full_name: Option<String>,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum OneOrMany<T> {
    Many(Vec<T>),
    One(T),
}

impl<T> OneOrMany<T> {
    fn into_vec(self) -> Vec<T> {
        match self {
            OneOrMany::Many(v) => v,
            OneOrMany::One(v) => vec![v],
        }
    }
}

pub fn gather() -> Result<Vec<DeviceUserRequest>> {
    let out = run(std::process::Command::new("powershell").args([
        "-NoProfile",
        "-Command",
        "Get-LocalUser | Select-Object Name,SID,FullName,Enabled | ConvertTo-Json",
    ]))?;
    let out = out.trim();
    if out.is_empty() {
        return Ok(Vec::new());
    }
    let raw: OneOrMany<RawUser> = serde_json::from_str(out)?;
    Ok(raw
        .into_vec()
        .into_iter()
        .map(|u| DeviceUserRequest {
            id: u.sid.value,
            username: Some(u.name),
            name: u.full_name.filter(|s| !s.is_empty()),
            home: None,
        })
        .collect())
}
