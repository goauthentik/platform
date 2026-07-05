use env_filter::{Filter, FilteredLog};
use eventlog::EventLog;
use log::LevelFilter;

pub fn init_log(name: &str, filter: Filter) {
    match EventLog::new(name, log::Level::Trace) {
        Ok(inner) => {
            log::set_boxed_logger(Box::new(FilteredLog::new(inner, filter)))
                .map(|()| log::set_max_level(LevelFilter::Trace))
                .unwrap_or_else(|_| eprintln!("Failed to initialize Windows Event Log"));
        }
        Err(e) => eprintln!("Failed to initialize Windows Event Log: {e:?}"),
    }
}
