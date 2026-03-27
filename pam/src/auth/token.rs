use authentik_sys::generated::sys_auth::system_auth_token_client::SystemAuthTokenClient;
use authentik_sys::generated::sys_auth::{TokenAuthRequest, TokenAuthResponse};
use authentik_sys::grpc::SysdBridge;
use pam::constants::PamResultCode;

fn validate_token_auth_response(
    username: &str,
    response: &TokenAuthResponse,
) -> Result<(), PamResultCode> {
    if !response.successful {
        return Err(PamResultCode::PAM_AUTH_ERR);
    }

    let token_username = response
        .token
        .as_ref()
        .ok_or(PamResultCode::PAM_AUTH_ERR)?
        .preferred_username
        .as_str();
    if username != token_username {
        log::warn!(
            "User mismatch: token={:#?}, expected={:#?}",
            token_username,
            username
        );
        return Err(PamResultCode::PAM_USER_UNKNOWN);
    }

    Ok(())
}

pub fn auth_token(
    username: String,
    token: String,
    bridge: impl SysdBridge,
) -> Result<TokenAuthResponse, PamResultCode> {
    let response = match bridge.grpc_request(async |ch| {
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

    validate_token_auth_response(&username, &response)?;
    log::debug!("Got valid token: {response:#?}");
    Ok(response)
}

#[cfg(test)]
mod tests {
    use authentik_sys::generated::agent::Token;
    use pam::constants::PamResultCode;

    use super::validate_token_auth_response;
    use authentik_sys::generated::sys_auth::TokenAuthResponse;

    fn response(successful: bool, preferred_username: Option<&str>) -> TokenAuthResponse {
        TokenAuthResponse {
            successful,
            token: preferred_username.map(|username| Token {
                preferred_username: username.to_owned(),
                iss: String::new(),
                sub: String::new(),
                aud: Vec::new(),
                exp: None,
                nbf: None,
                iat: None,
                jti: String::new(),
            }),
            session_id: "session-id".to_owned(),
        }
    }

    #[test]
    fn rejects_unsuccessful_responses() {
        assert_eq!(
            validate_token_auth_response("alice", &response(false, Some("alice"))),
            Err(PamResultCode::PAM_AUTH_ERR)
        );
    }

    #[test]
    fn rejects_missing_token_payloads() {
        assert_eq!(
            validate_token_auth_response("alice", &response(true, None)),
            Err(PamResultCode::PAM_AUTH_ERR)
        );
    }

    #[test]
    fn rejects_tokens_for_different_users() {
        assert_eq!(
            validate_token_auth_response("alice", &response(true, Some("bob"))),
            Err(PamResultCode::PAM_USER_UNKNOWN)
        );
    }

    #[test]
    fn accepts_valid_token_responses() {
        assert_eq!(
            validate_token_auth_response("alice", &response(true, Some("alice"))),
            Ok(())
        );
    }
}
