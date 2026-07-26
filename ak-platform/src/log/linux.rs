use eyre::Context;
use eyre::Result;
use log::LevelFilter;
use log::Log;
use simplelog::{Config, WriteLogger};
use std::fs::File;
use syslog::BasicLogger;
use syslog::Formatter3164;

pub fn init_log(name: &str) -> Result<Box<dyn Log>> {
    let formatter = Formatter3164 {
        process: name.into(),
        ..Default::default()
    };
    return match syslog::unix(formatter) {
        Ok(logger) => Ok(Box::new(BasicLogger::new(logger))),
        Err(e) => {
            eprintln!("unable to connect to syslog: {e:?}");
            return build_file_log(format!("/var/log/authentik/{}.log", name));
        }
    };
}

fn build_file_log(path: String) -> Result<Box<dyn Log>> {
    let file = File::create(path.clone()).context(format!("Failed to open {path}"))?;
    Ok(WriteLogger::new(
        LevelFilter::Trace,
        Config::default(),
        file,
    ))
}
