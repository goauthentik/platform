use std::io::{IsTerminal, stdout};

use eyre::Result;
use crate::string::PlatformString;
use env_filter::FilteredLog;
pub use log::LevelFilter;
use log::Log;
use simplelog::{Config, TermLogger};

#[cfg(target_os = "macos")]
pub mod macos;

#[cfg(target_os = "linux")]
pub mod linux;

#[cfg(target_os = "windows")]
pub mod windows;

#[cfg(unix)]
pub mod unix;

pub struct LogBuilder {
    name: PlatformString,
    filter: Option<String>,
    allow_platform: bool,
    force_stdout: bool,
    default_level: LevelFilter,
}

impl LogBuilder {
    pub fn new(name: PlatformString) -> Self {
        LogBuilder {
            name,
            filter: None,
            allow_platform: true,
            force_stdout: false,
            default_level: LevelFilter::Trace,
        }
    }

    pub fn allow_platform(mut self, allow_platform: bool) -> Self {
        self.allow_platform = allow_platform;
        self
    }

    pub fn force_stdout(mut self, force_stdout: bool) -> Self {
        self.force_stdout = force_stdout;
        self
    }

    pub fn with_filter<T: ToString>(mut self, filter: T) -> Self {
        self.filter = Some(filter.to_string());
        self
    }

    pub fn default_level(mut self, level: LevelFilter) -> Self {
        self.default_level = level;
        self
    }

    fn build_filter(&self) -> env_filter::Filter {
        let mut builder = env_filter::Builder::new();
        let src = self
            .filter
            .clone()
            .map(|s| s.to_owned())
            .unwrap_or_else(|| self.default_level.as_str().to_string());
        builder.parse(&src);
        builder.build()
    }

    fn get_stdout_logger(&self) -> Box<dyn Log> {
        TermLogger::new(
            LevelFilter::Trace,
            Config::default(),
            simplelog::TerminalMode::Stderr,
            simplelog::ColorChoice::Auto,
        )
    }

    fn get_platform_logger(&self) -> Result<Box<dyn Log>> {
        #[cfg(target_os = "macos")]
        return Ok(macos::init_log(&self.name.clone().for_current().to_owned()));
        #[cfg(target_os = "linux")]
        linux::init_log(&self.name.for_current());
        #[cfg(target_os = "windows")]
        windows::init_log(&self.name.for_current());
    }

    pub fn enable(&self) {
        let filter = self.build_filter();
        let inner = match (self.allow_platform && should_switch()) || self.force_stdout {
            true => self.get_stdout_logger(),
            false => self.get_platform_logger(),
        };
        log::set_boxed_logger(Box::new(FilteredLog::new(inner, filter)))
            .map(|()| log::set_max_level(self.default_level))
            .unwrap_or_else(|_| eprintln!("Failed to setup logger"));
    }
}

pub fn set_log_level(level: LevelFilter) {
    log::set_max_level(level);
}

pub fn should_switch() -> bool {
    if stdout().is_terminal() {
        return false;
    }
    #[cfg(debug_assertions)]
    return false;
    #[cfg(not(debug_assertions))]
    return true;
}
