use pam::{
    constants::{PAM_PROMPT_ECHO_OFF, PamFlag, PamResultCode},
    conv::Conv,
    items::User,
    module::PamHandle,
    pam_try,
};
use std::ffi::CStr;

use crate::{
    ENV_SESSION_ID,
    auth::{
        interactive::auth_interactive,
        token::{auth_token, decode_token},
    },
    config::Config,
    pam_env::pam_put_env,
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

    let username = match pam_try!(pamh.get_item::<User>()) {
        Some(u) => match String::from_utf8(u.to_bytes().to_vec()) {
            Ok(uu) => uu,
            Err(e) => {
                log::warn!("failed to decode user: {}", e);
                return PamResultCode::PAM_AUTH_ERR;
            }
        },
        None => {
            log::warn!("No user");
            return PamResultCode::PAM_AUTH_ERR;
        }
    };
    log::debug!("got username: {}", username);
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
    let password = pam_try!(conv.send(PAM_PROMPT_ECHO_OFF, "authentik Password: "));
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
    pam_try!(pam_put_env(pamh, ENV_SESSION_ID, id.to_owned().as_str()));

    if password.starts_with(PW_PREFIX) {
        log::debug!("Token authentication");
        let raw_token = password.replace(PW_PREFIX, "");
        let decoded = pam_try!(decode_token(raw_token));
        let token = match auth_token(config, username, decoded.token.to_owned()) {
            Ok(t) => t,
            Err(e) => return e,
        };
        session_data.token = decoded.token;
        session_data.expiry = token.claims.exp;
        session_data.local_socket = decoded.local_socket;
        match pam_put_env(
            pamh,
            "AUTHENTIK_CLI_SOCKET",
            session_data.local_socket.to_owned().as_str(),
        ) {
            Ok(t) => t,
            Err(e) => {
                log::warn!("Failed to set env");
                return e;
            }
        };
        pam_try!(_write_session_data(id, session_data));
        return PamResultCode::PAM_SUCCESS;
    } else {
        log::debug!("Interactive authentication");
        session_data.token = hash_token(&password.to_owned());
        pam_try!(_write_session_data(id, session_data));
        return auth_interactive(username, &password, &conv);
    }
}
