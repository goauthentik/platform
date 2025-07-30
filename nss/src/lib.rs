mod generated;
mod group;
mod passwd;
mod shadow;

use authentik_sys::logger::{init_log, log_hook};
use ctor::{ctor, dtor};
use group::AuthentikGroupHooks;
use libnss::{interop::Response, libnss_group_hooks, libnss_passwd_hooks, libnss_shadow_hooks};
use passwd::AuthentikPasswdHooks;
use shadow::AuthentikShadowHooks;
use tonic::{Code, Status};

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

fn grpc_status_to_nss_response<T>(status: Status) -> Response<T> {
    match status.code() {
        Code::NotFound => Response::NotFound,
        _ => Response::Unavail,
    }
}
