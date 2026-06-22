use log::LevelFilter;
use oslog::OsLogger;

pub fn init_log(name: &str) {
    OsLogger::new(name)
        .level_filter(LevelFilter::Trace)
        .init()
        .unwrap();
}
