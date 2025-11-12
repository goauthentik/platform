mod group;
mod passwd;
mod shadow;

use authentik_sys::logger::{exit_log, init_log, log_hook, set_log_level};
use ctor::{ctor, dtor};
use group::AuthentikGroupHooks;
use libnss::{libnss_group_hooks, libnss_passwd_hooks, libnss_shadow_hooks};
use log::LevelFilter;
use passwd::AuthentikPasswdHooks;
use shadow::AuthentikShadowHooks;

libnss_passwd_hooks!(authentik, AuthentikPasswdHooks);
libnss_shadow_hooks!(authentik, AuthentikShadowHooks);
libnss_group_hooks!(authentik, AuthentikGroupHooks);

#[ctor]
fn ctor() {
    init_log("libnss-authentik");
    // With NSS we don't have a good way to configure log level dynamically
    // we could read it from /etc/authentik
    set_log_level(LevelFilter::Warn);
    log_hook("ctor");
}

#[dtor]
fn dtor() {
    log_hook("dtor");
    exit_log();
}
