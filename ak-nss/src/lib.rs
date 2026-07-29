pub mod backend;
mod error;
mod group;
pub mod mapping;
mod passwd;
mod shadow;

use ak_platform::log::LevelFilter;
use ak_platform::log::LogBuilder;
use ak_platform::log::unix::log_hook;
use ak_platform::string::PlatformString;
use ctor::ctor;
use dtor::dtor;
use libnss::{libnss_group_hooks, libnss_passwd_hooks, libnss_shadow_hooks};

pub struct AuthentikNSS {}

libnss_passwd_hooks!(authentik, AuthentikNSS);
libnss_shadow_hooks!(authentik, AuthentikNSS);
libnss_group_hooks!(authentik, AuthentikNSS);

#[ctor(unsafe)]
fn ctor() {
    // With NSS we don't have a good way to configure log level dynamically
    // we could read it from /etc/authentik
    LogBuilder::new(PlatformString::new_with_default("libnss-authentik"))
        .default_level(LevelFilter::Warn)
        .allow_platform(true)
        .allow_stdout(false)
        .enable();
    log_hook("ctor");
}

#[dtor(unsafe)]
fn dtor() {
    log_hook("dtor");
}
