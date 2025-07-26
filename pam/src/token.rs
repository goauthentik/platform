extern crate jwks;
extern crate pam;
extern crate reqwest;
extern crate simplelog;

use jsonwebtoken::{TokenData, Validation, decode, decode_header};
use jwks::Jwks;
use pam::constants::{PamResultCode};
use serde::{Deserialize, Serialize};
use tokio::runtime::Runtime;
use crate::config::Config;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub preferred_username: String,
}

pub fn auth_token(config: Config, username: String, token: String) -> PamResultCode {
    let jwks = Runtime::new()
        .unwrap()
        .block_on(Jwks::from_jwks_url(format!("{}/application/o/{}/jwks/", config.authentik_url, config.app_slug)))
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
        log::warn!(target: "pam_authentik::auth_token", "User mismatch: token={:#?}, expected={:#?}", decoded_token.claims, username);
        return PamResultCode::PAM_USER_UNKNOWN;
    }
    return PamResultCode::PAM_SUCCESS;
}
