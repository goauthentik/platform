#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
use linux as imp;

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
use macos as imp;

#[cfg(target_os = "windows")]
mod windows;
#[cfg(target_os = "windows")]
use windows as imp;

#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
mod other;
#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
use other as imp;

use authentik_client::models::HardwareRequest;
use eyre::Result;

/// Gathers manufacturer/model/serial/CPU/memory facts for the current host.
pub fn gather() -> Result<HardwareRequest> {
    imp::gather()
}

/// Hardware serial number, also used for enrollment and JWT `iss` claims.
pub fn serial() -> Result<String> {
    imp::serial()
}
