use env_filter::{Filter, FilteredLog};
use eventlog::EventLog;
use eyre::Result;
use log::Log;

pub fn init_log(name: &str) -> Result<Box<dyn Log>> {
    EventLog::new(name, log::Level::Trace)
}
