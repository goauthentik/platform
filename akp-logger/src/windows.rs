use eventlog::init;

pub fn init_log(name: &str) {
    eventlog::init(name, log::Level::Trace).unwrap();
}
