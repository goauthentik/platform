extern crate jwks;
extern crate pam;
extern crate reqwest;
extern crate simplelog;
use simplelog::*;
use std::env;

use jsonwebtoken::{TokenData, Validation, decode, decode_header};
use jwks::Jwks;
use pam::constants::{PAM_PROMPT_ECHO_OFF, PamFlag, PamResultCode};
use pam::conv::Conv;
use pam::module::{PamHandle, PamHooks};
use pam::pam_try;
use serde::{Deserialize, Serialize};
use std::ffi::CStr;
use std::fs::File;
use tokio::runtime::Runtime;

struct PAMAuthentik;
pam::pam_hooks!(PAMAuthentik);

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub preferred_username: String,
}

fn auth_token(username: String, token: String) -> PamResultCode {
    let jwks_url = "http://authentik:9000/application/o/authentik-pam/jwks/";
    let jwks = Runtime::new()
        .unwrap()
        .block_on(Jwks::from_jwks_url(jwks_url))
        .unwrap();
    // get the kid from jwt
    log::debug!(target: "pam_authentik::auth_token", "Got JWT {}", token);
    let header = decode_header(&token).expect("jwt header should be decoded");
    let kid = header.kid.as_ref().expect("jwt header should have a kid");
    let jwk = jwks.keys.get(kid).expect("jwt refer to a unknown key id");
    let mut validation = Validation::new(jsonwebtoken::Algorithm::RS256);
    validation.set_audience(&["authentik-pam"]);
    let decoded_token: TokenData<Claims> =
        decode::<Claims>(&token, &jwk.decoding_key, &validation).expect("jwt should be valid");
    log::debug!(target: "pam_authentik::auth_token", "Got valid token: {:#?}", decoded_token.claims);
    if username != decoded_token.claims.preferred_username {
        return PamResultCode::PAM_USER_UNKNOWN;
    }
    return PamResultCode::PAM_SUCCESS;
}

fn init() {
    CombinedLogger::init(vec![WriteLogger::new(
        LevelFilter::Trace,
        Config::default(),
        File::options()
            .append(true)
            .create(true)
            .open("/var/log/authentik/pam.log")
            .unwrap(),
    )])
    .unwrap();
}

impl PamHooks for PAMAuthentik {
    fn sm_open_session(_pamh: &mut PamHandle, _args: Vec<&CStr>, _flags: PamFlag) -> PamResultCode {
        // init();
        log::debug!("sm_open_session");
        PamResultCode::PAM_SUCCESS
    }

    fn sm_authenticate(pamh: &mut PamHandle, args: Vec<&CStr>, _flags: PamFlag) -> PamResultCode {
        init();
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
        if password.unwrap_or("").starts_with("\u{200b}") {
            log::debug!(target: "pam_authentik::sm_authenticate", "Password has token marker");
            return auth_token(username, password.unwrap().replace("\u{200b}", ""));
        }
        PamResultCode::PAM_SUCCESS
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
