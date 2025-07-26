mod token;
mod logger;
mod interactive;
mod config;

extern crate jwks;
extern crate pam;
extern crate reqwest;
extern crate simplelog;

use std::env;
use pam::constants::{PAM_PROMPT_ECHO_OFF, PamFlag, PamResultCode};
use pam::conv::Conv;
use pam::module::{PamHandle, PamHooks};
use pam::pam_try;
use std::ffi::CStr;
use token::auth_token;
use logger::init_log;
use interactive::auth_interactive;
use ctor::ctor;
use config::Config;

struct PAMAuthentik;
pam::pam_hooks!(PAMAuthentik);

#[ctor]
fn init() {
    init_log();
}

impl PamHooks for PAMAuthentik {
    fn sm_open_session(_pamh: &mut PamHandle, _args: Vec<&CStr>, _flags: PamFlag) -> PamResultCode {
        log::debug!("sm_open_session");
        PamResultCode::PAM_SUCCESS
    }

    fn sm_authenticate(pamh: &mut PamHandle, args: Vec<&CStr>, _flags: PamFlag) -> PamResultCode {
        let config = Config::from_file("/etc/authentik/pam.yaml").expect("Failed to load config");

        log::debug!(target: "pam_authentik::sm_authenticate", "init");
        log::debug!(target: "pam_authentik::sm_authenticate", "debug args {:?}", args);
        log::debug!(target: "pam_authentik::sm_authenticate", "debug env {:?}", env::vars());
        let username = pamh.get_item::<pam::items::User>().unwrap().unwrap();
        let username = String::from_utf8(username.to_bytes().to_vec()).unwrap();
        log::debug!(target: "pam_authentik::sm_authenticate", "user: {}", username);
        let conv = match pamh.get_item::<Conv>() {
            Ok(Some(conv)) => conv,
            Ok(None) => {
                unreachable!("No conv available");
            }
            Err(err) => {
                log::debug!("Couldn't get pam_conv");
                return err;
            }
        };
        log::debug!(target: "pam_authentik::sm_authenticate", "Started conv");
        let password = pam_try!(conv.send(PAM_PROMPT_ECHO_OFF, "authentik Password: "));
        let password = match password {
            Some(password) => Some(pam_try!(password.to_str(), PamResultCode::PAM_AUTH_ERR)),
            None => {
                unreachable!("No password");
            }
        };
        log::debug!(target: "pam_authentik::sm_authenticate", "{:#?}", password);
        if password.unwrap_or("").starts_with("\u{200b}") {
            log::debug!(target: "pam_authentik::sm_authenticate", "Password has token marker");
            return auth_token(config, username, password.unwrap().replace("\u{200b}", ""));
        } else {
            log::debug!(target: "pam_authentik::sm_authenticate", "Interactive authentication");
            return auth_interactive(username, password.unwrap(), &conv);
        }
    }

    fn sm_setcred(_pamh: &mut PamHandle, _args: Vec<&CStr>, _flags: PamFlag) -> PamResultCode {
        log::debug!("sm_setcred");
        PamResultCode::PAM_SUCCESS
    }

    fn acct_mgmt(_pamh: &mut PamHandle, _args: Vec<&CStr>, _flags: PamFlag) -> PamResultCode {
        log::debug!("acct_mgmt");
        PamResultCode::PAM_SUCCESS
    }
}
