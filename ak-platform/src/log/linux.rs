use std::fs::File;

use env_filter::{Filter, FilteredLog};
use log::LevelFilter;
use simplelog::{Config, WriteLogger};
use syslog::BasicLogger;
use syslog::{Facility, Formatter3164};

pub fn init_log(name: &str, filter: Filter) {
    let formatter = Formatter3164 {
        facility: Facility::LOG_USER,
        hostname: None,
        process: name.into(),
        pid: std::process::id(),
    };
    let inner: Box<dyn log::Log> = match syslog::unix(formatter) {
        Ok(logger) => Box::new(BasicLogger::new(logger)),
        Err(e) => {
            eprintln!("unable to connect to syslog: {e:?}");
            match build_file_log(format!("/var/log/authentik/{}.log", name)) {
                Some(logger) => logger,
                None => return,
            }
        }
    };
    log::set_boxed_logger(Box::new(FilteredLog::new(inner, filter)))
        .map(|()| log::set_max_level(LevelFilter::Trace))
        .unwrap_or(());
}

fn build_file_log(path: String) -> Option<Box<dyn log::Log>> {
    let file = File::create(path).ok()?;
    Some(WriteLogger::new(
        LevelFilter::Trace,
        Config::default(),
        file,
    ))
}
