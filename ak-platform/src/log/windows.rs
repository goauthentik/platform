use eventlog::EventLog;
use eyre::Result;
use log::Log;

pub fn init_log(name: &str) -> Result<Box<dyn Log>> {
    match EventLog::new(name, log::Level::Trace) {
        Ok(l) => Ok(Box::new(l)),
        Err(e) => Err(e.into()),
    }
}
