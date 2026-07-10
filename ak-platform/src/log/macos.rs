use env_filter::{Filter, FilteredLog};
use log::LevelFilter;
use oslog::OsLogger;

pub fn init_log(name: &str, filter: Filter) {
    let inner = OsLogger::new(name).level_filter(LevelFilter::Trace);
    log::set_boxed_logger(Box::new(FilteredLog::new(inner, filter)))
        .map(|()| log::set_max_level(LevelFilter::Trace))
        .unwrap_or(());
}
