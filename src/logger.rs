use libc::{getegid, geteuid, getgid, getuid};
use log::LevelFilter;
use syslog::BasicLogger;
use syslog::{Facility, Formatter3164};

pub fn init_log(name: &str) {
    let formatter = Formatter3164 {
        facility: Facility::LOG_USER,
        hostname: None,
        process: name.into(),
        pid: std::process::id(),
    };
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

pub fn set_log_level(level: LevelFilter) {
    log::set_max_level(level);
}

pub fn exit_log() {}

pub fn log_hook(name: &str) {
    let pid = std::process::id();
    let ppid = std::os::unix::process::parent_id();
    let uid = unsafe { getuid() };
    let gid = unsafe { getgid() };
    let euid = unsafe { geteuid() };
    let egid = unsafe { getegid() };
    log::debug!("{name}, pid: {pid}, ppid: {ppid}, uid/gid: {uid}:{gid}, euid/egid: {euid}:{egid}");
}
