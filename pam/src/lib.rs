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
use crate::logger::log_hook_with_args;
use crate::session::close_session_impl;
use crate::session::open_session_impl;
use ctor::{ctor, dtor};
use pam::constants::{PamFlag, PamResultCode};
use pam::module::{PamHandle, PamHooks};
use std::ffi::CStr;

pub const DATA_CLIENT: &str = "client";
pub const ENV_SESSION_ID: &str = "AUTHENTIK_SESSION_ID";

struct PAMAuthentik;
pam::pam_hooks!(PAMAuthentik);

#[ctor]
fn ctor() {
    init_log();
    log_hook("ctor");
}

#[dtor]
fn dtor() {
    log_hook("dtor");
}

impl PamHooks for PAMAuthentik {
    fn sm_authenticate(pamh: &mut PamHandle, args: Vec<&CStr>, flags: PamFlag) -> PamResultCode {
        log_hook_with_args("sm_authenticate", args.clone());
        return authenticate_impl(pamh, args, flags);
    }

    fn sm_open_session(pamh: &mut PamHandle, args: Vec<&CStr>, flags: PamFlag) -> PamResultCode {
        log_hook_with_args("sm_open_session", args.clone());
        return open_session_impl(pamh, args, flags);
    }

    fn sm_close_session(pamh: &mut PamHandle, args: Vec<&CStr>, flags: PamFlag) -> PamResultCode {
        log_hook_with_args("sm_close_session", args.clone());
        return close_session_impl(pamh, args, flags);
    }

    fn sm_setcred(_pamh: &mut PamHandle, args: Vec<&CStr>, _flags: PamFlag) -> PamResultCode {
        log_hook_with_args("sm_setcred", args.clone());
        PamResultCode::PAM_SUCCESS
    }

    fn acct_mgmt(_pamh: &mut PamHandle, args: Vec<&CStr>, _flags: PamFlag) -> PamResultCode {
        log_hook_with_args("acct_mgmt", args.clone());
        PamResultCode::PAM_SUCCESS
    }
}
