use libc::{getegid, geteuid, getgid, getuid};

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
