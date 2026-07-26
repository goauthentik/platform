use log::{LevelFilter, Log};
use oslog::OsLogger;

pub fn init_log(name: &str) -> Box<dyn Log> {
    Box::new(OsLogger::new(name).level_filter(LevelFilter::Trace))
}
