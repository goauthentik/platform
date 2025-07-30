use libc::{getegid, geteuid, getgid, getuid};
use log::LevelFilter;
use syslog::BasicLogger;
use syslog::{Facility, Formatter3164};

pub fn init_log() {
    let formatter = Formatter3164 {
        facility: Facility::LOG_USER,
        hostname: None,
        process: "libnss-authentik".into(),
        pid: std::process::id(),
    };
    let logger = match syslog::unix(formatter) {
        Ok(logger) => logger,
        Err(e) => {
            println!("impossible to connect to syslog: {:?}", e);
            return;
        }
    };
    log::set_boxed_logger(Box::new(BasicLogger::new(logger)))
        .map(|()| log::set_max_level(LevelFilter::Trace))
        .expect("Failed to setup logger");
}

pub fn log_hook(name: &str) {
    let pid = std::process::id();
    let ppid = std::os::unix::process::parent_id();
    let uid = unsafe { getuid() };
    let gid = unsafe { getgid() };
    let euid = unsafe { geteuid() };
    let egid = unsafe { getegid() };
    log::debug!(
        "{}, pid: {}, ppid: {}, uid/gid: {}:{}, euid/egid: {}:{}",
        name,
        pid,
        ppid,
        uid,
        gid,
        euid,
        egid
    );
}
