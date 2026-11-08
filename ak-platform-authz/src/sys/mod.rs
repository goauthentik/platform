use ak_platform::string::PlatformString;
use eyre::Result;

use crate::Operation;

#[cfg(target_os = "macos")]
pub mod macos;

#[cfg(target_os = "linux")]
pub mod linux;

#[cfg(target_os = "windows")]
pub mod windows;

pub async fn prompt(operation: Operation, msg: PlatformString) -> Result<bool> {
    #[cfg(target_os = "macos")]
    return macos::prompt(operation, msg.for_current()).await;
    #[cfg(target_os = "linux")]
    return linux::prompt(operation, msg).await;
    #[cfg(target_os = "windows")]
    return windows::prompt(operation, msg).await;
}
