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

use std::sync::{LazyLock, RwLock};

use authentik_client::models::HardwareRequest;
use eyre::Result;

/// Lets an embedding host supply the serial directly, bypassing the
/// platform-specific lookup. Only consulted on macOS — the only platform
/// where a native lookup isn't otherwise available in embedded contexts.
static STATIC_SERIAL: LazyLock<RwLock<Option<String>>> = LazyLock::new(|| RwLock::new(None));

/// Pass `None` to clear the override.
pub fn set_static_serial(serial: Option<String>) {
    if let Ok(mut guard) = STATIC_SERIAL.write() {
        *guard = serial;
    }
}

pub(crate) fn static_serial() -> Option<String> {
    STATIC_SERIAL.read().ok().and_then(|guard| guard.clone())
}

pub fn gather() -> Result<HardwareRequest> {
    imp::gather()
}

/// Also used for enrollment and JWT `iss` claims, where a full [`gather`]
/// would be wasteful.
pub fn serial() -> Result<String> {
    imp::serial()
}
