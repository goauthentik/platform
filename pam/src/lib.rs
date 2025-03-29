extern crate pam;
extern crate reqwest;
extern crate jwks;

use pam::constants::{PamFlag, PamResultCode, PAM_PROMPT_ECHO_OFF};
use pam::conv::Conv;
use pam::module::{PamHandle, PamHooks};
use reqwest::blocking::Client;
use reqwest::StatusCode;
use std::ffi::CStr;
use std::time::Duration;
use pam::pam_try;
use jsonwebtoken::{decode, decode_header, TokenData, Validation};
use jwks::Jwks;
use serde::{Deserialize, Serialize};
use tokio::runtime::Runtime;

struct PAMAuthentik;
pam::pam_hooks!(PAMAuthentik);

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
}


fn auth_token(pamh: &mut PamHandle)  -> PamResultCode {
    let conv = match pamh.get_item::<Conv>() {
        Ok(Some(conv)) => conv,
        Ok(None) => {
            unreachable!("No conv available");
        }
        Err(err) => {
            println!("Couldn't get pam_conv");
            return err;
        }
    };
    let token = match pam_try!(conv.send(PAM_PROMPT_ECHO_OFF, "ak-cli-token-prompt:")) {
        Some(token) => Some(pam_try!(token.to_str(), PamResultCode::PAM_AUTH_ERR)),
        None => {
            return PamResultCode::PAM_AUTH_ERR;
        },
    };
    let jwks_url = "http://authentik:9000/application/o/authentik-pam/jwks/";
    let jwks = Runtime::new().unwrap().block_on(Jwks::from_jwks_url(jwks_url)).unwrap();
    let token = token.expect("token should be string");
    // get the kid from jwt
    println!("Got JWT {}", token);
    let header = decode_header(token).expect("jwt header should be decoded");
    let kid = header.kid.as_ref().expect("jwt header should have a kid");

    let jwk = jwks.keys.get(kid).expect("jwt refer to a unknown key id");

    let mut validation = Validation::new(jsonwebtoken::Algorithm::RS256);
    validation.set_audience(&["authentik-pam"]);
    let decoded_token: TokenData<Claims> =
        decode::<Claims>(token, &jwk.decoding_key, &validation).expect("jwt should be valid");
    println!("Got valid token: {:#?}", decoded_token.claims);
    return PamResultCode::PAM_SUCCESS
}

impl PamHooks for PAMAuthentik {

    // This function performs the task of authenticating the user.
    fn sm_authenticate(pamh: &mut PamHandle, args: Vec<&CStr>, _flags: PamFlag) -> PamResultCode {
        let user = pam_try!(pamh.get_user(None));

        if user.ends_with("@ak-token") {
            return auth_token(pamh);
        }

        // let password = pam_try!(conv.send(PAM_PROMPT_ECHO_OFF, "Word, yo: "));
        // let password = match password {
        //     Some(password) => Some(pam_try!(password.to_str(), PamResultCode::PAM_AUTH_ERR)),
        //     None => None,
        // };
        // println!("Got a password {:?}", password);
        // let status = pam_try!(
        //     get_url(url, &user, password),
        //     PamResultCode::PAM_AUTH_ERR
        // );

        // if !status.is_success() {
        //     println!("HTTP Error: {}", status);
        //     return PamResultCode::PAM_AUTH_ERR;
        // }

        PamResultCode::PAM_SUCCESS
    }

    fn sm_setcred(_pamh: &mut PamHandle, _args: Vec<&CStr>, _flags: PamFlag) -> PamResultCode {
        println!("set credentials");
        PamResultCode::PAM_SUCCESS
    }

    fn acct_mgmt(_pamh: &mut PamHandle, _args: Vec<&CStr>, _flags: PamFlag) -> PamResultCode {
        println!("account management");
        PamResultCode::PAM_SUCCESS
    }
}

fn get_url(url: &str, user: &str, password: Option<&str>) -> reqwest::Result<StatusCode> {
    let client = Client::builder().timeout(Duration::from_secs(15)).build()?;
    client
        .get(url)
        .basic_auth(user, password)
        .send()
        .map(|r| r.status())
}
