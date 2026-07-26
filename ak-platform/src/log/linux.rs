use std::fs::File;

use env_filter::{Filter, FilteredLog};
use log::LevelFilter;
use simplelog::{Config, WriteLogger};
use syslog::BasicLogger;
use syslog::{Facility, Formatter3164};


pub fn init_log(name: &str, filter: Filter) -> Result<Box<dyn Log>> {
    let formatter = Formatter3164 {
        facility: Facility::LOG_USER,
        hostname: None,
        process: name.into(),
        pid: std::process::id(),
    };
    return match syslog::unix(formatter) {
        Ok(logger) => Box::new(BasicLogger::new(logger)),
        Err(e) => {
            eprintln!("unable to connect to syslog: {e:?}");
            match build_file_log(format!("/var/log/authentik/{}.log", name)) {
                Some(logger) => logger,
                None => return,
            }
        }
    };
}

fn build_file_log(path: String) -> Option<Box<dyn log::Log>> {
    let file = File::create(path).ok()?;
    Some(WriteLogger::new(
        LevelFilter::Trace,
        Config::default(),
        file,
    ))
}
