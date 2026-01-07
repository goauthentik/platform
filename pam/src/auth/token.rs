use ::prost::Message;
use authentik_sys::generated::ssh::SshTokenAuthentication;
use authentik_sys::generated::sys_auth::system_auth_token_client::SystemAuthTokenClient;
use authentik_sys::generated::sys_auth::{TokenAuthRequest, TokenAuthResponse};
use authentik_sys::grpc::grpc_request;
use base64::{Engine, prelude::BASE64_STANDARD};
use pam::constants::PamResultCode;

pub fn decode_token(token: String) -> Result<SshTokenAuthentication, PamResultCode> {
    let raw = match BASE64_STANDARD.decode(token) {
        Ok(t) => t,
        Err(e) => {
            log::warn!("Failed to base64 decode token: {e}");
            return Err(PamResultCode::PAM_AUTH_ERR);
        }
    };

    let msg = match SshTokenAuthentication::decode(&*raw) {
        Ok(t) => t,
        Err(e) => {
            log::warn!("failed to decode message: {e}");
            return Err(PamResultCode::PAM_AUTH_ERR);
        }
    };
    Ok(msg)
}

pub fn auth_token(username: String, token: String) -> Result<TokenAuthResponse, PamResultCode> {
    let response = match grpc_request(async |ch| {
        return Ok(SystemAuthTokenClient::new(ch)
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
    let token_username = response
        .token
        .clone()
        .ok_or(PamResultCode::PAM_AUTH_ERR)?
        .preferred_username;
    if username != token_username {
        log::warn!(
            "User mismatch: token={:#?}, expected={:#?}",
            token_username,
            username
        );
        return Err(PamResultCode::PAM_USER_UNKNOWN);
    }
    Ok(response)
}
