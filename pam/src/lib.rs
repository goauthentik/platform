mod auth;
mod generated;
mod pam_env;
mod session;

extern crate jwks;
extern crate pam;
extern crate reqwest;

use crate::auth::authenticate_impl;
use crate::session::close_session_impl;
use crate::session::open_session_impl;
use authentik_sys::logger::init_log;
use authentik_sys::logger::log_hook;
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
    init_log("libpam-authentik");
    log_hook("ctor");
}

#[dtor]
fn dtor() {
    log_hook("dtor");
}

impl PamHooks for PAMAuthentik {
    fn sm_authenticate(pamh: &mut PamHandle, args: Vec<&CStr>, flags: PamFlag) -> PamResultCode {
        log_hook("sm_authenticate");
        pam_check_service!(pamh);
        authenticate_impl(pamh, args, flags)
    }

    fn sm_open_session(pamh: &mut PamHandle, args: Vec<&CStr>, flags: PamFlag) -> PamResultCode {
        log_hook("sm_open_session");
        pam_check_service!(pamh);
        open_session_impl(pamh, args, flags)
    }

    fn sm_close_session(pamh: &mut PamHandle, args: Vec<&CStr>, flags: PamFlag) -> PamResultCode {
        log_hook("sm_close_session");
        pam_check_service!(pamh);
        close_session_impl(pamh, args, flags)
    }

    fn sm_setcred(pamh: &mut PamHandle, _args: Vec<&CStr>, _flags: PamFlag) -> PamResultCode {
        log_hook("sm_setcred");
        pam_check_service!(pamh);
        PamResultCode::PAM_SUCCESS
    }

    fn acct_mgmt(pamh: &mut PamHandle, _args: Vec<&CStr>, _flags: PamFlag) -> PamResultCode {
        log_hook("acct_mgmt");
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

#[macro_export]
macro_rules! pam_try_log {
    ($r:expr, $l:expr) => {
        match $r {
            Ok(t) => t,
            Err(e) => {
                log::warn!($l);
                return e;
            }
        }
    };
    ($r:expr, $l:expr, $e:expr) => {
        match $r {
            Ok(t) => t,
            Err(_) => {
                log::warn!($l);
                return $e;
            }
        }
    };
}
