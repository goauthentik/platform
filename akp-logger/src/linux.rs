use log::LevelFilter;
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
            println!("impossible to connect to syslog: {e:?}");
            return;
        }
    };
    log::set_boxed_logger(Box::new(BasicLogger::new(logger)))
        .map(|()| log::set_max_level(LevelFilter::Trace))
        .unwrap_or(());
}
