use libc::{getegid, geteuid, getgid, getuid};
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

pub fn log_hook(name: &str) {
    let pid = std::process::id();
    let ppid = std::os::unix::process::parent_id();
    let uid = unsafe { getuid() };
    let gid = unsafe { getgid() };
    let euid = unsafe { geteuid() };
    let egid = unsafe { getegid() };
    log::debug!("{} init, pid: {}, ppid: {}, uid/gid: {}:{}, euid/egid: {}:{}", name, pid, ppid, uid, gid, euid, egid);
}
