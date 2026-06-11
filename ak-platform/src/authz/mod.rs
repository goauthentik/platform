use std::error::Error;

use crate::platform::string::PlatformString;

#[cfg(target_os = "macos")]
pub mod macos;

#[cfg(target_os = "linux")]
pub mod linux;


pub async fn prompt(msg: PlatformString) -> Result<bool, Box<dyn Error>> {
    #[cfg(target_os = "macos")]
    return macos::prompt(msg).await;
    #[cfg(target_os = "linux")]
    return linux::prompt(msg).await;
}
