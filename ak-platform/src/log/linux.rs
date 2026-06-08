use std::fs::File;

use log::LevelFilter;
use simplelog::{Config, WriteLogger};
use syslog::BasicLogger;
use syslog::{Facility, Formatter3164};

pub fn init_log(name: &str) {
    let formatter = Formatter3164 {
        facility: Facility::LOG_USER,
        hostname: None,
        process: name.into(),
        pid: std::process::id(),
    };
    let logger = match syslog::unix(formatter) {
        Ok(logger) => logger,
        Err(e) => {
            init_file_log(format!("/var/log/authentik/{}.log", name));
            log::warn!("unable to connect to syslog: {e:?}");
            return;
        }
    };
    log::set_boxed_logger(Box::new(BasicLogger::new(logger)))
        .map(|()| log::set_max_level(LevelFilter::Trace))
        .unwrap_or(());
}

fn init_file_log(path: String) {
    let file = match File::create(path) {
        Ok(f) => f,
        Err(_) => return,
    };
    let _ = WriteLogger::init(LevelFilter::Trace, Config::default(), file);
}
