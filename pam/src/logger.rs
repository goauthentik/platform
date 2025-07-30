use std::ffi::CStr;

use libc::{getegid, geteuid, getgid, getuid};
use log::LevelFilter;
use syslog::BasicLogger;
use syslog::{Facility, Formatter3164};

pub fn init_log() {
    let formatter = Formatter3164 {
        facility: Facility::LOG_USER,
        hostname: None,
        process: "ak_pam".into(),
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
        "{name} init, pid: {pid}, ppid: {ppid}, uid/gid: {uid}:{gid}, euid/egid: {euid}:{egid}"
    );
}

pub fn log_hook_with_args(name: &str, args: Vec<&CStr>) {
    let arg_str: Vec<&str> = args
        .iter()
        .map(|c| c.to_str().unwrap_or("<invalid>"))
        .collect::<_>();
    log_hook(
        format!("{} (args {})", name, arg_str.join(", "))
            .to_owned()
            .as_str(),
    );
}

#[macro_export]
macro_rules! pam_try_log {
    ($r:expr, $l:expr) => {
        match $r {
            Ok(t) => t,
            Err(e) => {
                log::warn!($l);
                return e;
            }
        }
    };
    ($r:expr, $l:expr, $e:expr) => {
        match $r {
            Ok(t) => t,
            Err(_) => {
                log::warn!($l);
                return $e;
            }
        }
    };
}
