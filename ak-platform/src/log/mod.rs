use std::io::{IsTerminal, stdout};

use crate::string::PlatformString;
use env_filter::FilteredLog;
use eyre::Result;
pub use log::LevelFilter;
use log::Log;
use simplelog::{ConfigBuilder, TermLogger};

#[cfg(target_os = "macos")]
pub mod macos;

#[cfg(target_os = "linux")]
pub mod linux;

#[cfg(target_os = "windows")]
pub mod windows;

#[cfg(unix)]
pub mod unix;

pub mod void;
pub struct LogBuilder {
    name: PlatformString,
    filter: Vec<(String, LevelFilter)>,
    allow_platform: bool,
    allow_stdout: bool,
    force_stdout: bool,
    default_level: LevelFilter,
}

impl LogBuilder {
    pub fn new(name: PlatformString) -> Self {
        LogBuilder {
            name,
            filter: vec![],
            allow_platform: true,
            allow_stdout: true,
            force_stdout: false,
            default_level: LevelFilter::Trace,
        }
    }

    pub fn allow_platform(mut self, allow_platform: bool) -> Self {
        self.allow_platform = allow_platform;
        self
    }

    pub fn allow_stdout(mut self, allow_stdout: bool) -> Self {
        self.allow_stdout = allow_stdout;
        self
    }

    pub fn force_stdout(mut self, force_stdout: bool) -> Self {
        self.force_stdout = force_stdout;
        self
    }

    pub fn with_default_filters(mut self) -> Self {
        self.filter.push(("h2".to_owned(), LevelFilter::Warn));
        self.filter.push(("tonic".to_owned(), LevelFilter::Warn));
        self.filter
            .push(("hyper_util".to_owned(), LevelFilter::Warn));
        self.filter.push(("reqwest".to_owned(), LevelFilter::Warn));
        self.filter
            .push(("tracing::span".to_owned(), LevelFilter::Warn));
        self
    }

    pub fn with_filter<T: ToString>(mut self, module: T, level: LevelFilter) -> Self {
        self.filter.push((module.to_string(), level));
        self
    }

    pub fn default_level(mut self, level: LevelFilter) -> Self {
        self.default_level = level;
        self
    }

    fn build_filter(&self) -> env_filter::Filter {
        let mut builder = env_filter::Builder::new();
        builder.filter_level(self.default_level);
        for (_mod, filter) in self.filter.clone() {
            builder.filter(Some(&_mod), filter);
        }
        builder.build()
    }

    fn get_stdout_logger(&self) -> Box<dyn Log> {
        TermLogger::new(
            LevelFilter::Trace,
            ConfigBuilder::new()
                .set_location_level(LevelFilter::Error)
                .set_thread_level(LevelFilter::Error)
                .set_thread_mode(simplelog::ThreadLogMode::Both)
                .build(),
            simplelog::TerminalMode::Stderr,
            simplelog::ColorChoice::Auto,
        )
    }

    fn get_platform_logger(&self) -> Result<Box<dyn Log>> {
        #[cfg(target_os = "macos")]
        return macos::init_log(&self.name.clone().for_current().to_owned());
        #[cfg(target_os = "linux")]
        return linux::init_log(&self.name.clone().for_current());
        #[cfg(target_os = "windows")]
        return windows::init_log(&self.name.clone().for_current());
    }

    pub fn build(&self) -> (Box<dyn Log>, LevelFilter) {
        let filter = self.build_filter();
        let max_level = self
            .filter
            .iter()
            .map(|(_, level)| *level)
            .fold(self.default_level, std::cmp::max);
        let inner: Box<dyn Log> = if self.allow_stdout && (env_interactive() || self.force_stdout) {
            self.get_stdout_logger()
        } else {
            match self.get_platform_logger() {
                Ok(l) => l,
                Err(_) => {
                    if self.allow_stdout {
                        self.get_stdout_logger()
                    } else {
                        void::VoidLogger::new()
                    }
                }
            }
        };
        (Box::new(FilteredLog::new(inner, filter)), max_level)
    }

    pub fn enable(&self) {
        let (logger, max_level) = self.build();
        log::set_boxed_logger(logger)
            .map(|()| log::set_max_level(max_level))
            .unwrap_or_else(|_| eprintln!("Failed to setup logger"));
    }
}

pub fn set_log_level(level: LevelFilter) {
    log::set_max_level(level);
}

pub fn env_interactive() -> bool {
    if stdout().is_terminal() {
        return true;
    }
    #[cfg(debug_assertions)]
    return true;
    #[cfg(not(debug_assertions))]
    return false;
}

#[cfg(test)]
mod test {
    use log::log_enabled;

    use crate::{log::LogBuilder, string::PlatformString};

    #[test]
    fn test_filter() {
        LogBuilder::new(PlatformString::new())
            .force_stdout(true)
            .default_level(log::LevelFilter::Info)
            .with_filter("ak_platform::log", log::LevelFilter::Trace)
            .enable();
        assert!(log_enabled!(log::Level::Trace));
    }
}
