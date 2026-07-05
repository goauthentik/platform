use std::io::IsTerminal;

use crate::string::PlatformString;
use env_filter::FilteredLog;
use simplelog::{Config, TermLogger};

pub use log::LevelFilter;

#[cfg(target_os = "macos")]
pub mod macos;

#[cfg(target_os = "linux")]
pub mod linux;

#[cfg(target_os = "windows")]
pub mod windows;

#[cfg(unix)]
pub mod unix;

pub fn init_log(name: PlatformString) {
    init_log_inner(name, None);
}

pub fn init_log_with_filter(name: PlatformString, directives: &str) {
    init_log_inner(name, Some(directives));
}

fn init_log_inner(name: PlatformString, directives: Option<&str>) {
    if !should_switch() {
        return init_log_interactive_with_filter(directives);
    }
    let filter = build_filter(directives);
    #[cfg(target_os = "macos")]
    macos::init_log(&name.for_current(), filter);
    #[cfg(target_os = "linux")]
    linux::init_log(&name.for_current(), filter);
    #[cfg(target_os = "windows")]
    windows::init_log(&name.for_current(), filter);
}

pub fn set_log_level(level: LevelFilter) {
    log::set_max_level(level);
}

pub fn init_log_interactive() {
    init_log_interactive_with_filter(None);
}

pub fn init_log_interactive_with_filter(directives: Option<&str>) {
    let filter = build_filter(directives);
    let inner = TermLogger::new(
        LevelFilter::Trace,
        Config::default(),
        simplelog::TerminalMode::Stderr,
        simplelog::ColorChoice::Auto,
    );
    log::set_boxed_logger(Box::new(FilteredLog::new(inner, filter)))
        .map(|()| log::set_max_level(LevelFilter::Trace))
        .unwrap_or_else(|_| eprintln!("Failed to setup terminal logger"));
}

fn build_filter(directives: Option<&str>) -> env_filter::Filter {
    let mut builder = env_filter::Builder::new();
    let src = directives
        .map(|s| s.to_owned())
        .unwrap_or_else(|| "trace".to_owned());
    builder.parse(&src);
    builder.build()
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
