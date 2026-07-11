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

use authentik_client::models::OperatingSystemRequest;
use eyre::Result;

/// Gathers OS family/name/version/arch for the current host.
pub fn gather() -> Result<OperatingSystemRequest> {
    imp::gather()
}
