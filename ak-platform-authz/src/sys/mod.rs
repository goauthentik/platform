use ak_platform::net::server::proc_info::ProcInfo;
use ak_platform::prelude::*;
use ak_platform::string::PlatformString;

#[cfg(target_os = "macos")]
pub mod macos;

#[cfg(target_os = "linux")]
pub mod linux;

#[cfg(target_os = "windows")]
pub mod windows;

pub struct AuthorizationRequest {
    pub msg: PlatformString,
    pub proc_info: Option<ProcInfo>,
    pub profile: Option<String>,
    // pub user_info: AuthentikClaims,
}

pub async fn prompt(req: AuthorizationRequest) -> Result<bool> {
    #[cfg(target_os = "macos")]
    return macos::prompt(req).await;
    #[cfg(target_os = "linux")]
    return linux::prompt(req).await;
    #[cfg(target_os = "windows")]
    return windows::prompt(req).await;
}
