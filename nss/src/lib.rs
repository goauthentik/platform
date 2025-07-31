mod group;
mod passwd;
mod shadow;

use authentik_sys::logger::{init_log, log_hook};
use ctor::{ctor, dtor};
use group::AuthentikGroupHooks;
use libnss::{libnss_group_hooks, libnss_passwd_hooks, libnss_shadow_hooks};
use passwd::AuthentikPasswdHooks;
use shadow::AuthentikShadowHooks;

libnss_passwd_hooks!(authentik, AuthentikPasswdHooks);
libnss_shadow_hooks!(authentik, AuthentikShadowHooks);
libnss_group_hooks!(authentik, AuthentikGroupHooks);

#[ctor]
fn ctor() {
    init_log("libnss-authentik");
    log_hook("ctor");
}

#[dtor]
fn dtor() {
    log_hook("dtor");
}
