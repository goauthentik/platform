use crate::platform::string::PlatformString;
use log::LevelFilter;

#[cfg(target_os = "macos")]
mod macos;

#[cfg(target_os = "linux")]
mod linux;

#[cfg(target_os = "windows")]
mod windows;

pub fn init_log(name: PlatformString) {
    #[cfg(target_os = "macos")]
    macos::init_log(&name.for_current());
    #[cfg(target_os = "linux")]
    linux::init_log(&name.for_current());
    #[cfg(target_os = "windows")]
    windows::init_log(&name.for_current());
}

pub fn set_log_level(level: LevelFilter) {
    log::set_max_level(level);
}
