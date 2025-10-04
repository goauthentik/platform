mod auth;
mod pam_env;
mod session;
mod session_data;

use crate::auth::authenticate_authorize_impl;
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
        let svc = pam_try_log!(get_service(pamh), "Failed to get service");
        match svc.as_str() {
            "sshd" => authenticate_impl(pamh, args, flags),
            "sudo" => authenticate_authorize_impl(pamh, args, flags),
            "sudo-i" => authenticate_authorize_impl(pamh, args, flags),
            _ => PamResultCode::PAM_IGNORE
        }
    }

    fn sm_open_session(pamh: &mut PamHandle, args: Vec<&CStr>, flags: PamFlag) -> PamResultCode {
        log_hook("sm_open_session");
        let svc = pam_try_log!(get_service(pamh), "Failed to get service");
        match svc.as_str() {
            "sshd" => open_session_impl(pamh, args, flags),
            _ => PamResultCode::PAM_IGNORE
        }
    }

    fn sm_close_session(pamh: &mut PamHandle, args: Vec<&CStr>, flags: PamFlag) -> PamResultCode {
        log_hook("sm_close_session");
        let svc = pam_try_log!(get_service(pamh), "Failed to get service");
        match svc.as_str() {
            "sshd" => close_session_impl(pamh, args, flags),
            _ => PamResultCode::PAM_IGNORE
        }
    }

    fn sm_setcred(pamh: &mut PamHandle, _args: Vec<&CStr>, _flags: PamFlag) -> PamResultCode {
        log_hook("sm_setcred");
        let svc = pam_try_log!(get_service(pamh), "Failed to get service");
        match svc.as_str() {
            "sshd" => PamResultCode::PAM_SUCCESS,
            _ => PamResultCode::PAM_IGNORE
        }
    }

    fn acct_mgmt(pamh: &mut PamHandle, _args: Vec<&CStr>, _flags: PamFlag) -> PamResultCode {
        log_hook("acct_mgmt");
        let svc = pam_try_log!(get_service(pamh), "Failed to get service");
        match svc.as_str() {
            "sshd" => PamResultCode::PAM_SUCCESS,
            _ => PamResultCode::PAM_IGNORE
        }
    }
}

pub fn get_service(pamh: &mut PamHandle) -> Result<String, PamResultCode> {
   match pamh.get_item::<Service>() {
        Ok(u) => match u {
            Some(u) => match String::from_utf8(u.to_bytes().to_vec()) {
                Ok(uu) => {
                    let svc = uu.to_owned();
                    return Ok(svc);
                },
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
    }
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
