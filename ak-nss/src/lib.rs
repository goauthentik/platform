mod group;
mod passwd;
mod shadow;

#[cfg(target_vendor = "apple")]
use ak_platform::log::LevelFilter;
use ak_platform::log::unix::log_hook;
use ak_platform::log::{init_log, set_log_level};
use ak_platform::string::PlatformString;
use ctor::ctor;
use dtor::dtor;
use group::AuthentikGroupHooks;
use libnss::{libnss_group_hooks, libnss_passwd_hooks, libnss_shadow_hooks};
use passwd::AuthentikPasswdHooks;
use shadow::AuthentikShadowHooks;

libnss_passwd_hooks!(authentik, AuthentikPasswdHooks);
libnss_shadow_hooks!(authentik, AuthentikShadowHooks);
libnss_group_hooks!(authentik, AuthentikGroupHooks);

#[ctor(unsafe)]
fn ctor() {
    init_log(PlatformString::new_with_default("libnss-authentik"));
    // With NSS we don't have a good way to configure log level dynamically
    // we could read it from /etc/authentik
    set_log_level(LevelFilter::Warn);
    log_hook("ctor");
}

#[dtor(unsafe)]
fn dtor() {
    log_hook("dtor");
}
