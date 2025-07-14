use simplelog::*;
use std::fs::File;

pub fn init_log() {
    CombinedLogger::init(vec![WriteLogger::new(
        LevelFilter::Trace,
        Config::default(),
        File::options()
            .append(true)
            .create(true)
            .open("/var/log/authentik/pam.log")
            .unwrap(),
    )])
    .unwrap();
}
