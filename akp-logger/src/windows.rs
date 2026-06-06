pub fn init_log(name: &str) {
    eventlog::init(name, log::Level::Trace).unwrap();
    let logger = match syslog::unix(formatter) {
        Ok(logger) => logger,
        Err(e) => {
            println!("impossible to connect to syslog: {e:?}");
            return;
        }
    };
    log::set_boxed_logger(Box::new(BasicLogger::new(logger)))
        .map(|()| log::set_max_level(LevelFilter::Trace))
        .unwrap_or(());
}
