use eventlog::init;

pub fn init_log(name: &str) {
    init(name, log::Level::Trace).unwrap();
}
