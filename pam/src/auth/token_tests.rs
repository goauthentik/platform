use authentik_sys::generated::agent::Token;
use authentik_sys::generated::sys_auth::TokenAuthResponse;
use pam::constants::PamResultCode;

use super::validate_token_auth_response;

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
