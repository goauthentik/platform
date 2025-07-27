mod auth;
mod config;
mod generated;
mod logger;
mod pam_env;
mod session;

extern crate jwks;
extern crate pam;
extern crate reqwest;

use crate::auth::authenticate_impl;
use crate::logger::init_log;
use crate::logger::log_hook;
use crate::session::open_session_impl;
use ctor::ctor;
use pam::constants::{PamFlag, PamResultCode};
use pam::module::{PamHandle, PamHooks};
use std::ffi::CStr;

pub const ENV_SESSION_ID: &str = "AUTHENTIK_SESSION_ID";

struct PAMAuthentik;
pam::pam_hooks!(PAMAuthentik);

#[ctor]
fn init() {
    init_log();
}

impl PamHooks for PAMAuthentik {
    fn sm_authenticate(pamh: &mut PamHandle, args: Vec<&CStr>, flags: PamFlag) -> PamResultCode {
        log_hook("sm_authenticate");
        return authenticate_impl(pamh, args, flags);
    }

    fn sm_open_session(pamh: &mut PamHandle, args: Vec<&CStr>, flags: PamFlag) -> PamResultCode {
        log_hook("sm_open_session");
        return open_session_impl(pamh, args, flags);
    }

    fn sm_setcred(_pamh: &mut PamHandle, _args: Vec<&CStr>, _flags: PamFlag) -> PamResultCode {
        log_hook("sm_setcred");
        PamResultCode::PAM_SUCCESS
    }

    fn acct_mgmt(_pamh: &mut PamHandle, _args: Vec<&CStr>, _flags: PamFlag) -> PamResultCode {
        log_hook("acct_mgmt");
        PamResultCode::PAM_SUCCESS
    }
}
