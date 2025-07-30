use authentik_sys::config::Config;
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
    session::{_generate_id, _write_session_data, SessionData, hash_token},
};

pub mod interactive;
pub mod token;

pub const PW_PREFIX: &str = "\u{200b}";

pub fn authenticate_impl(
    pamh: &mut PamHandle,
    _args: Vec<&CStr>,
    _flags: PamFlag,
) -> PamResultCode {
    let config = Config::from_file("/etc/authentik/host.yaml").expect("Failed to load config");

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
    let password = pam_try_log!(
        conv.send(PAM_PROMPT_ECHO_OFF, "authentik Password: "),
        "failed to send prompt"
    );
    let password = match password {
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

    let id = _generate_id().to_string();
    let mut session_data = SessionData {
        username: username.to_string(),
        token: password.to_owned(),
        expiry: -1,
        local_socket: "".to_owned(),
    };
    pam_try_log!(
        pam_put_env(pamh, ENV_SESSION_ID, id.to_owned().as_str()),
        "failed to set session_id env"
    );

    if password.starts_with(PW_PREFIX) {
        log::debug!("Token authentication");
        let raw_token = password.replace(PW_PREFIX, "");
        let decoded = pam_try_log!(decode_token(raw_token), "failed to decode token");
        let token = match auth_token(config, username, decoded.token.to_owned()) {
            Ok(t) => t,
            Err(e) => return e,
        };
        session_data.token = decoded.token;
        session_data.expiry = token.claims.exp;
        session_data.local_socket = decoded.local_socket;
        pam_try_log!(
            pam_put_env(
                pamh,
                "AUTHENTIK_CLI_SOCKET",
                session_data.local_socket.to_owned().as_str(),
            ),
            "Failed to set env"
        );
        pam_try_log!(
            _write_session_data(id, session_data),
            "failed to write session data"
        );
        PamResultCode::PAM_SUCCESS
    } else {
        log::debug!("Interactive authentication");
        session_data.token = hash_token(password);
        pam_try_log!(
            _write_session_data(id, session_data),
            "failed to write session data"
        );
        auth_interactive(username, password, &conv)
    }
}
