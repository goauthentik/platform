use pam::{
    constants::{PAM_PROMPT_ECHO_OFF, PamFlag, PamResultCode},
    conv::Conv,
    items::User,
    module::PamHandle,
};
use std::ffi::CStr;

use crate::{
    ENV_SESSION_ID,
    auth::{
        interactive::auth_interactive,
        token::{auth_token, decode_token},
    },
    pam_env::pam_put_env,
    pam_try_log,
    session_data::{_write_session_data, SessionData},
};

pub mod authorize;
pub mod interactive;
pub mod token;

pub const PW_PREFIX: &str = "\u{200b}";
pub const PW_PROMPT: &str = "authentik Password: ";

pub fn authenticate_impl(
    pamh: &mut PamHandle,
    _args: Vec<&CStr>,
    _flags: PamFlag,
) -> PamResultCode {
    let username = match pamh.get_item::<User>() {
        Ok(u) => match u {
            Some(u) => match String::from_utf8(u.to_bytes().to_vec()) {
                Ok(uu) => uu,
                Err(e) => {
                    log::warn!("failed to decode user: {e}");
                    return PamResultCode::PAM_AUTH_ERR;
                }
            },
            None => {
                log::warn!("No user");
                return PamResultCode::PAM_AUTH_ERR;
            }
        },
        Err(e) => {
            log::warn!("failed to get user");
            return e;
        }
    };
    log::debug!("got username: '{username}'");
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
    log::debug!("Started conv");
    let password = match pam_try_log!(
        conv.send(PAM_PROMPT_ECHO_OFF, PW_PROMPT),
        "failed to send prompt"
    ) {
        Some(password) => match password.to_str() {
            Ok(t) => t,
            Err(_) => {
                log::warn!("failed to convert password");
                return PamResultCode::PAM_AUTH_ERR;
            }
        },
        None => {
            log::warn!("No password!");
            return PamResultCode::PAM_AUTH_ERR;
        }
    };

    let mut session_data = SessionData {
        username: username.to_string(),
        token: password.to_owned(),
        expiry: -1,
        local_socket: "".to_owned(),
    };
    let session_id: String;

    if password.starts_with(PW_PREFIX) {
        log::debug!("Token authentication");
        let raw_token = password.replace(PW_PREFIX, "");
        let decoded = pam_try_log!(decode_token(raw_token), "failed to decode token");
        let token_res = match auth_token(username, decoded.token.to_owned()) {
            Ok(t) => t,
            Err(e) => return e,
        };
        session_data.expiry = token_res.token.unwrap().exp.unwrap().seconds;
        session_data.local_socket = decoded.local_socket;
        session_id = token_res.session_id;
    } else {
        log::debug!("Interactive authentication");
        let int_res = match auth_interactive(username, password.to_owned(), &conv) {
            Ok(ss) => ss,
            Err(code) => return code,
        };
        session_id = int_res.session_id;
    }
    if !session_data.local_socket.is_empty() {
        pam_try_log!(
            pam_put_env(
                pamh,
                "AUTHENTIK_CLI_SOCKET",
                session_data.local_socket.to_owned().as_str(),
            ),
            "Failed to set env"
        );
    }
    pam_try_log!(
        _write_session_data(session_id.clone(), session_data),
        "failed to write session data"
    );
    pam_try_log!(
        pam_put_env(pamh, ENV_SESSION_ID, session_id.to_owned().as_str()),
        "failed to set session_id env"
    );
    PamResultCode::PAM_SUCCESS
}
