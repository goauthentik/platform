mod config;
mod generated;
mod interactive;
mod logger;
mod session;
mod token;

extern crate jwks;
extern crate pam;
extern crate reqwest;
extern crate simplelog;

use config::Config;
use ctor::ctor;
use interactive::auth_interactive;
use logger::init_log;
use pam::constants::{PAM_PROMPT_ECHO_OFF, PamFlag, PamResultCode};
use pam::conv::Conv;
use pam::module::{PamHandle, PamHooks};
use pam::pam_try;
use session::open_session_impl;
use std::env;
use std::ffi::{CStr, CString};
use token::auth_token;

use crate::session::SessionData;

struct PAMAuthentik;
pam::pam_hooks!(PAMAuthentik);

#[ctor]
fn init() {
    init_log();
}

impl PamHooks for PAMAuthentik {
    fn sm_authenticate(pamh: &mut PamHandle, _args: Vec<&CStr>, _flags: PamFlag) -> PamResultCode {
        let config = Config::from_file("/etc/authentik/pam.yaml").expect("Failed to load config");

        log::debug!(target: "pam_authentik::sm_authenticate", "init");
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
        let auth_info = "test_data".to_string();
        match pamh.set_data("my_key", Box::new(auth_info)) {
            Ok(_) => log::debug!("DEBUG: Data set successfully"),
            Err(e) => log::debug!("DEBUG: Failed to set data: {:?}", e),
        }

        unsafe { env::set_var("qwerqerqewr", "yeah bruv") };

        if password.unwrap_or("").starts_with("\u{200b}") ||
            password.unwrap_or("").starts_with("ey") {
            log::debug!(target: "pam_authentik::sm_authenticate", "Password has token marker");
            let token = password.unwrap().replace("\u{200b}", "");
            // pam_try!(pamh.set_data("token", Box::new(token.to_owned())));
            return auth_token(config, username, token);
        } else {
            log::debug!(target: "pam_authentik::sm_authenticate", "Interactive authentication");
            // pam_try!(pamh.set_data("token", Box::new(password.to_owned())));
            return auth_interactive(username, password.unwrap(), &conv);
        }
    }

    fn sm_open_session(pamh: &mut PamHandle, args: Vec<&CStr>, flags: PamFlag) -> PamResultCode {
        log::debug!("sm_open_session");
        return open_session_impl(pamh, args, flags);
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
