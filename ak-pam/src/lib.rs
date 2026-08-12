mod auth;
mod dir;
mod logger;
mod pam_env;
mod session;
mod session_data;

use crate::auth::authenticate_impl;
use crate::auth::authorize::authenticate_authorize_impl;
use crate::logger::prelude;
use crate::session::close_session_impl;
use crate::session::open_session_impl;
use ak_platform::log::LogBuilder;
use ak_platform::log::unix::log_hook;
use ak_platform::string::PlatformString;
use ctor::ctor;
use dtor::dtor;
use eyre::Context;
use pam::constants::PAM_TEXT_INFO;
use pam::constants::{PamFlag, PamResultCode};
use pam::conv::Conv;
use pam::items::Service;
use pam::items::User;
use pam::module::{PamHandle, PamHooks};
use std::ffi::CStr;
use std::fmt::Display;

pub const ENV_SESSION_ID: &str = "AUTHENTIK_SESSION_ID";

struct PAMAuthentik;
pam::pam_hooks!(PAMAuthentik);

#[ctor(unsafe)]
fn ctor() {
    LogBuilder::new(PlatformString::new_with_default("libpam-authentik"))
        .allow_platform(true)
        .allow_stdout(false)
        .enable();
    log_hook("ctor");
}

#[dtor(unsafe)]
fn dtor() {
    log_hook("dtor");
}

impl PamHooks for PAMAuthentik {
    fn sm_authenticate(pamh: &mut PamHandle, args: Vec<&CStr>, flags: PamFlag) -> PamResultCode {
        prelude("sm_authenticate", pamh, args.clone(), flags);
        let svc = match get_service(pamh) {
            Ok(svc) => svc,
            Err(c) => return c.code,
        };
        match svc.as_str() {
            "sudo" => authenticate_authorize_impl("sudo"),
            "sudo-i" => authenticate_authorize_impl("sudo-i"),
            _ => authenticate_impl(pamh),
        }
        .map_err(|e| {
            tracing::warn!("Error in sm_authenticate: {e:?}");
        })
        .unwrap_or(PamResultCode::PAM_AUTH_ERR)
    }

    fn sm_open_session(pamh: &mut PamHandle, args: Vec<&CStr>, flags: PamFlag) -> PamResultCode {
        prelude("sm_open_session", pamh, args.clone(), flags);
        open_session_impl(pamh)
            .map_err(|e| {
                tracing::warn!("Error in sm_open_session: {e:?}");
            })
            .unwrap_or(PamResultCode::PAM_SESSION_ERR)
    }

    fn sm_close_session(pamh: &mut PamHandle, args: Vec<&CStr>, flags: PamFlag) -> PamResultCode {
        prelude("sm_close_session", pamh, args.clone(), flags);
        close_session_impl(pamh)
            .map_err(|e| {
                tracing::warn!("Error in sm_close_session: {e:?}");
            })
            .unwrap_or(PamResultCode::PAM_SESSION_ERR)
    }

    fn sm_setcred(pamh: &mut PamHandle, args: Vec<&CStr>, flags: PamFlag) -> PamResultCode {
        prelude("sm_setcred", pamh, args.clone(), flags);
        let svc = match get_service(pamh) {
            Ok(svc) => svc,
            Err(c) => return c.code,
        };
        match svc.as_str() {
            "sshd" => PamResultCode::PAM_SUCCESS,
            _ => PamResultCode::PAM_IGNORE,
        }
    }

    fn acct_mgmt(pamh: &mut PamHandle, args: Vec<&CStr>, flags: PamFlag) -> PamResultCode {
        prelude("acct_mgmt", pamh, args.clone(), flags);
        let svc = match get_service(pamh) {
            Ok(svc) => svc,
            Err(c) => return c.code,
        };
        match svc.as_str() {
            "sshd" => PamResultCode::PAM_SUCCESS,
            _ => PamResultCode::PAM_IGNORE,
        }
    }
}

pub fn get_service(pamh: &mut PamHandle) -> Result<String, PamError> {
    match pamh.get_item::<Service>() {
        Ok(Some(u)) => match String::from_utf8(u.to_bytes().to_vec()) {
            Ok(uu) => {
                let svc = uu.to_owned();
                Ok(svc)
            }
            Err(e) => {
                tracing::warn!("failed to decode service: {e}");
                Err(PamResultCode::PAM_AUTH_ERR.into())
            }
        },
        Ok(None) => {
            tracing::warn!("No service");
            Err(PamResultCode::PAM_AUTH_ERR.into())
        }
        Err(e) => {
            tracing::warn!("failed to get service");
            Err(e.into())
        }
    }
}

pub fn pam_print_user(conv: &Conv<'_>, text: &str) {
    match conv.send(PAM_TEXT_INFO, text) {
        Ok(_) => {}
        Err(e) => {
            tracing::warn!("Failed to print text to user: {:?}", e);
        }
    }
}

#[derive(Debug)]
pub struct PamError {
    code: PamResultCode,
}

impl Display for PamError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "PAM Error code {:?}", self.code)
    }
}

impl From<PamResultCode> for PamError {
    fn from(value: PamResultCode) -> Self {
        PamError { code: value }
    }
}

impl std::error::Error for PamError {}

pub fn username(pamh: &mut PamHandle) -> Result<String, PamError> {
    match pamh.get_item::<User>() {
        Ok(Some(u)) => Ok(String::from_utf8(u.to_bytes().to_vec())
            .context("failed to decode user")
            .map_err(|e| {
                tracing::warn!("failed to convert username to utf8: {e:?}");
                PamError::from(PamResultCode::PAM_SESSION_ERR)
            })?),
        Ok(None) => {
            tracing::warn!("No user");
            Err(PamResultCode::PAM_SERVICE_ERR.into())
        }
        Err(e) => {
            tracing::warn!("failed to get user");
            Err(e.into())
        }
    }
}
