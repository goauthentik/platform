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
use pam::items::Service;
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
        pam_check_service!(pamh);
        authenticate_impl(pamh, args, flags)
    }

    fn sm_open_session(pamh: &mut PamHandle, args: Vec<&CStr>, flags: PamFlag) -> PamResultCode {
        log_hook_with_args("sm_open_session", args.clone());
        pam_check_service!(pamh);
        open_session_impl(pamh, args, flags)
    }

    fn sm_close_session(pamh: &mut PamHandle, args: Vec<&CStr>, flags: PamFlag) -> PamResultCode {
        log_hook_with_args("sm_close_session", args.clone());
        pam_check_service!(pamh);
        close_session_impl(pamh, args, flags)
    }

    fn sm_setcred(pamh: &mut PamHandle, args: Vec<&CStr>, _flags: PamFlag) -> PamResultCode {
        log_hook_with_args("sm_setcred", args.clone());
        pam_check_service!(pamh);
        PamResultCode::PAM_SUCCESS
    }

    fn acct_mgmt(pamh: &mut PamHandle, args: Vec<&CStr>, _flags: PamFlag) -> PamResultCode {
        log_hook_with_args("acct_mgmt", args.clone());
        pam_check_service!(pamh);
        PamResultCode::PAM_SUCCESS
    }
}

#[macro_export]
macro_rules! pam_check_service {
    ($h: expr) => {
        match check_service($h) {
            Ok(()) => {}
            Err(e) => {
                log::debug!("ignoring request for service");
                return e;
            }
        }
    };
}

pub fn check_service(pamh: &mut PamHandle) -> Result<(), PamResultCode> {
    let service = match pamh.get_item::<Service>() {
        Ok(u) => match u {
            Some(u) => match String::from_utf8(u.to_bytes().to_vec()) {
                Ok(uu) => uu,
                Err(e) => {
                    log::warn!("failed to decode user: {e}");
                    return Err(PamResultCode::PAM_AUTH_ERR);
                }
            },
            None => {
                log::warn!("No user");
                return Err(PamResultCode::PAM_AUTH_ERR);
            }
        },
        Err(e) => {
            log::warn!("failed to get user");
            return Err(e);
        }
    };
    log::debug!("Service: '{service}'");
    if ["sshd"].contains(&service.to_owned().as_str()) {
        return Ok(());
    }
    Err(PamResultCode::PAM_IGNORE)
}
