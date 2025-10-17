use std::time::Duration;

use libc::{getegid, geteuid, getgid, getuid};
use log::LevelFilter;
use sentry::ClientInitGuard;
use syslog::BasicLogger;
use syslog::{Facility, Formatter3164};

static mut SENTRY_CLIENT: Option<ClientInitGuard> = None;

pub fn init_log(name: &str) {
    let ver = option_env!("CARGO_PKG_VERSION").unwrap_or("foo");
    let rel_str = format!("ak-platform-{name}@{ver}");
    let _guard = sentry::init((
        "https://c83cdbb55c9bd568ecfa275932b6de17@o4504163616882688.ingest.us.sentry.io/4509208005312512",
        sentry::ClientOptions {
            release: Some(rel_str.into()),
            traces_sample_rate: 0.3,
            ..Default::default()
        },
    ));
    unsafe { SENTRY_CLIENT = Some(_guard) };

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
        .expect("Failed to setup logger");
}

pub fn exit_log() {
    if let Some(client) = sentry::Hub::current().client() {
        client.close(Some(Duration::from_secs(2)));
    }
}

pub fn log_hook(name: &str) {
    let pid = std::process::id();
    let ppid = std::os::unix::process::parent_id();
    let uid = unsafe { getuid() };
    let gid = unsafe { getgid() };
    let euid = unsafe { geteuid() };
    let egid = unsafe { getegid() };
    log::debug!("{name}, pid: {pid}, ppid: {ppid}, uid/gid: {uid}:{gid}, euid/egid: {euid}:{egid}");
}
