use ::prost::Message;
use authentik_sys::generated::agent::Token;
use authentik_sys::generated::pam::pam_client::PamClient;
use authentik_sys::generated::{
    grpc_request,
    pam::{PamAuthentication, TokenAuthRequest},
};
use base64::{Engine, prelude::BASE64_STANDARD};
use pam::constants::PamResultCode;

pub fn decode_token(token: String) -> Result<PamAuthentication, PamResultCode> {
    let raw = match BASE64_STANDARD.decode(token) {
        Ok(t) => t,
        Err(e) => {
            log::warn!("Failed to base64 decode token: {e}");
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

pub fn auth_token(username: String, token: String) -> Result<Token, PamResultCode> {
    let response = match grpc_request(async |ch| {
        return Ok(PamClient::new(ch)
            .token_auth(TokenAuthRequest {
                username: username.to_owned(),
                token: token.to_owned(),
            })
            .await?);
    }) {
        Ok(t) => t.into_inner(),
        Err(e) => {
            log::warn!("failed to validate token: {e}");
            return Err(PamResultCode::PAM_AUTH_ERR);
        }
    };

    if !response.successful {
        return Err(PamResultCode::PAM_AUTH_ERR);
    }

    log::debug!("Got valid token: {response:#?}");
    if username != response.token.clone().unwrap().preferred_username {
        log::warn!(
            "User mismatch: token={:#?}, expected={:#?}",
            response.token.unwrap(),
            username
        );
        return Err(PamResultCode::PAM_USER_UNKNOWN);
    }
    Ok(response.token.unwrap())
}
