//! The one `ak-sysd` call this DLL makes: capability discovery, cached in
//! HKLM so `SetUsageScenario` doesn't need the daemon reachable on every
//! logon-screen paint.

use eyre::Result;
use serde::{Deserialize, Serialize};
use winreg::enums::HKEY_LOCAL_MACHINE;

use ak_platform::generated::ping::capabilities_response::Capability;
use ak_platform::generated::ping::ping_client::PingClient;
use ak_platform::grpc::grpc_request;

/// HKLM key `sys_caps` caches its answer in. `e2e`'s harness seeds this same
/// key to turn on `debug`; keep the name and the `Capabilities` field names
/// in step with `e2e::harness`.
pub const CAPABILITIES_KEY: &str = "SOFTWARE\\authentik Security Inc.\\Platform\\Capabilities";

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct Capabilities {
    pub interactive_auth_available: bool,
    pub debug: bool,
}

pub fn sys_caps() -> Result<Capabilities> {
    let hklm = winreg::RegKey::predef(HKEY_LOCAL_MACHINE);
    let (key, _disp) = hklm.create_subkey(CAPABILITIES_KEY)?;

    if let Ok(caps) = key.decode() {
        return Ok(caps);
    }

    let response =
        grpc_request(async |ch| Ok(PingClient::new(ch).capabilities(()).await?))?.into_inner();
    let authia = Capability::AuthInteractive as i32;
    let caps = Capabilities {
        interactive_auth_available: response.capabilities.contains(&authia),
        debug: false,
    };
    key.encode(&caps)?;
    Ok(caps)
}
