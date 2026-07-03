use eyre::Result;
use ak_platform::string::PlatformString;

#[cfg(target_os = "macos")]
pub mod macos;

#[cfg(target_os = "linux")]
pub mod linux;

#[cfg(target_os = "windows")]
pub mod windows;

pub async fn prompt(msg: PlatformString) -> Result<bool> {
    #[cfg(target_os = "macos")]
    return macos::prompt(msg.for_current()).await;
    #[cfg(target_os = "linux")]
    return linux::prompt(msg).await;
    #[cfg(target_os = "windows")]
    return windows::prompt(msg).await;
}
