//! The `ak-sysd` calls this DLL makes. `sys_caps` is cached in HKLM so
//! `SetUsageScenario` does not need the daemon on every logon-screen paint.
//! `sys_auth_start_async`/`sys_auth_validate` live here rather than in
//! `ak_browser.exe`, whose service account cannot reach `ak-sysd`'s pipe
//! (`BROWSER_PRIVILEGE.md`).

use eyre::Result;
use serde::{Deserialize, Serialize};
use winreg::enums::HKEY_LOCAL_MACHINE;

use ak_platform::generated::ping::capabilities_response::Capability;
use ak_platform::generated::ping::ping_client::PingClient;
use ak_platform::generated::sys_auth::TokenAuthRequest;
use ak_platform::generated::sys_auth::system_auth_interactive_client::SystemAuthInteractiveClient;
use ak_platform::generated::sys_auth::system_auth_token_client::SystemAuthTokenClient;
use ak_platform::grpc::grpc_request;

/// `ak_ee_wcp_e2e::harness` seeds this same key to turn on `debug`; keep the
/// name and the `Capabilities` fields in step with it.
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

pub struct AuthStartAsync {
    pub url: String,
    pub header_token: String,
}

/// Starts an interactive sign-in: `url` is what `ak_browser.exe` opens, and
/// `header_token` what it injects on every request that page makes, so the
/// backend can tie them back to this session.
pub fn sys_auth_start_async() -> Result<AuthStartAsync> {
    let response = grpc_request(async |ch| {
        Ok(SystemAuthInteractiveClient::new(ch)
            .interactive_auth_async(())
            .await?)
    })?
    .into_inner();
    Ok(AuthStartAsync {
        url: response.url,
        header_token: response.header_token,
    })
}

/// Validates the token embedded in the sign-in redirect's URL. `None` covers
/// both an unextractable token and one `ak-sysd` rejects.
pub fn sys_auth_validate(url: &str) -> Result<Option<String>> {
    let Some(raw_token) = ak_ee_wcp_wire::extract_token(url) else {
        return Ok(None);
    };

    let response = grpc_request(async |ch| {
        Ok(SystemAuthTokenClient::new(ch)
            .token_auth(TokenAuthRequest {
                username: String::new(),
                token: raw_token.clone(),
            })
            .await?)
    })?
    .into_inner();

    if !response.successful {
        return Ok(None);
    }
    Ok(Some(
        response
            .token
            .map(|t| t.preferred_username)
            .unwrap_or_default(),
    ))
}
