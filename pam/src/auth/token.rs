extern crate jwks;
extern crate pam;
extern crate reqwest;

use ::prost::Message;
use authentik_sys::{config::Config, generated::pam::PamAuthentication};
use jsonwebtoken::{TokenData, Validation, decode, decode_header};
use jwks::Jwks;
use pam::constants::PamResultCode;
use serde::{Deserialize, Serialize};
use tokio::runtime::Runtime;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub preferred_username: String,
    pub exp: i64,
}

pub fn decode_token(token: String) -> Result<PamAuthentication, PamResultCode> {
    let raw = match hex::decode(token) {
        Ok(t) => t,
        Err(e) => {
            log::warn!("Failed to hex decode token: {e}");
            return Err(PamResultCode::PAM_AUTH_ERR);
        }
    };

    let msg = match PamAuthentication::decode(&*raw) {
        Ok(t) => t,
        Err(e) => {
            log::warn!("failed to decode message: {e}");
            return Err(PamResultCode::PAM_AUTH_ERR);
        }
    };
    Ok(msg)
}

pub fn auth_token(
    config: Config,
    username: String,
    token: String,
) -> Result<TokenData<Claims>, PamResultCode> {
    let rt = match Runtime::new() {
        Ok(rt) => rt,
        Err(e) => {
            log::warn!("Failed to create runtime: {e}");
            return Err(PamResultCode::PAM_SESSION_ERR);
        }
    };
    let jwks = match rt.block_on(Jwks::from_jwks_url(format!(
        "{}/application/o/{}/jwks/",
        config.authentik_url, config.app_slug
    ))) {
        Ok(res) => res,
        Err(e) => {
            log::warn!("failed to validate token: {e}");
            return Err(PamResultCode::PAM_SESSION_ERR);
        }
    };
    // get the kid from jwt
    let header = match decode_header(&token) {
        Ok(t) => t,
        Err(e) => {
            log::warn!("failed to decode JWT header: {e}");
            return Err(PamResultCode::PAM_AUTH_ERR);
        }
    };
    let kid = match header.kid.as_ref() {
        Some(t) => t,
        None => {
            log::warn!("failed to get JWT header kid");
            return Err(PamResultCode::PAM_AUTH_ERR);
        }
    };
    let jwk = match jwks.keys.get(kid) {
        Some(t) => t,
        None => {
            log::warn!("JWT refers to non-existent kid");
            return Err(PamResultCode::PAM_AUTH_ERR);
        }
    };
    let mut validation = Validation::new(jsonwebtoken::Algorithm::RS256);
    validation.set_audience(&["authentik-pam"]);
    let decoded_token: TokenData<Claims> =
        decode::<Claims>(&token, &jwk.decoding_key, &validation).expect("jwt should be valid");
    log::debug!("Got valid token: {:#?}", decoded_token.claims);
    if username != decoded_token.claims.preferred_username {
        log::warn!(
            "User mismatch: token={:#?}, expected={:#?}",
            decoded_token.claims,
            username
        );
        return Err(PamResultCode::PAM_USER_UNKNOWN);
    }
    Ok(decoded_token)
}
