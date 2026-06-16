use std::io::IsTerminal;

use crate::string::PlatformString;
use log::LevelFilter;
use simplelog::{Config, TermLogger};

#[cfg(target_os = "macos")]
pub mod macos;

#[cfg(target_os = "linux")]
pub mod linux;

#[cfg(target_os = "windows")]
pub mod windows;

#[cfg(unix)]
pub mod unix;

pub fn init_log(name: PlatformString) {
    if !should_switch() {
        return init_log_interactive();
    }
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

pub fn init_log_interactive() {
    TermLogger::init(
        LevelFilter::Trace,
        Config::default(),
        simplelog::TerminalMode::Stderr,
        simplelog::ColorChoice::Auto,
    )
    .unwrap_or_else(|_| eprintln!("Failed to setup terminal logger"));
}

pub fn should_switch() -> bool {
    if std::io::stdout().is_terminal() {
        return false;
    }
    #[cfg(debug_assertions)]
    return false;
    #[cfg(not(debug_assertions))]
    return true;
}
