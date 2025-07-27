use rand::Rng;
use std::{ffi::CStr, fs::File, io::Write, os::unix::fs::PermissionsExt};
use sha2::{Digest, Sha256};
use pam::{
    constants::{PAM_PROMPT_ECHO_OFF, PamFlag, PamResultCode},
    conv::Conv,
    module::PamHandle,
    pam_try,
};

use crate::{
    ENV_SESSION_ID,
    auth::{interactive::auth_interactive, token::auth_token},
    config::Config,
    pam_env::pam_put_env,
    session::SessionData,
};

pub mod interactive;
pub mod token;

pub const PW_PREFIX: &str = "\u{200b}";

pub fn authenticate_impl(
    pamh: &mut PamHandle,
    _args: Vec<&CStr>,
    _flags: PamFlag,
) -> PamResultCode {
    let config = Config::from_file("/etc/authentik/pam.yaml").expect("Failed to load config");

    let username = pamh.get_item::<pam::items::User>().unwrap().unwrap();
    let username = String::from_utf8(username.to_bytes().to_vec()).unwrap();
    log::debug!("user: {}", username);
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
        Some(password) => {
            match password.to_str() {
                Ok(t) => t,
                Err(_) => {
                    log::warn!("failed to convert password");
                    return PamResultCode::PAM_AUTH_ERR;
                }
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
    };
    pam_try!(pam_put_env(pamh, ENV_SESSION_ID, id.to_owned().as_str()));

    if password.starts_with(PW_PREFIX) {
        log::debug!("Token authentication");
        let raw_token = password.replace(PW_PREFIX, "");
        let token = match auth_token(config, username, raw_token.to_owned()) {
            Ok(t) => t,
            Err(e) => return e,
        };
        session_data.token = raw_token;
        session_data.expiry = token.claims.exp;
        pam_try!(_write_session_data(id, session_data));
        return PamResultCode::PAM_SUCCESS;
    } else {
        log::debug!("Interactive authentication");
        session_data.token = hash_token(&password.to_owned());
        pam_try!(_write_session_data(id, session_data));
        return auth_interactive(username, &password, &conv);
    }
}

pub fn _read_session_data(id: String) -> Result<SessionData, PamResultCode> {
    let path = format!("/tmp/.aksm-{}", id);
    let file = File::open(path).expect("Could not create file!");

    return match serde_json::from_reader(file) {
        Ok(t) => Ok(t),
        Err(e) => {
            log::warn!("failed to write session data: {}", e);
            return Err(PamResultCode::PAM_AUTH_ERR);
        }
    };
}

pub fn _write_session_data(id: String, data: SessionData) -> Result<(), PamResultCode> {
    let json_data = serde_json::to_string(&data).unwrap();
    let path = format!("/tmp/.aksm-{}", id);
    let mut file = File::create(path).expect("Could not create file!");

    let mut permissions = file.metadata().unwrap().permissions();
    permissions.set_mode(0o400);
    file.set_permissions(permissions).unwrap();

    return match file.write_all(json_data.as_bytes()) {
        Ok(_) => Ok(()),
        Err(e) => {
            log::warn!("failed to write session data: {}", e);
            return Err(PamResultCode::PAM_AUTH_ERR);
        }
    };
}

pub fn _generate_id() -> String {
    const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ\
                            abcdefghijklmnopqrstuvwxyz\
                            0123456789";
    const PASSWORD_LEN: usize = 30;
    let mut rng = rand::rng();

    return (0..PASSWORD_LEN)
        .map(|_| {
            let idx = rng.random_range(0..CHARSET.len());
            CHARSET[idx] as char
        })
        .collect();
}

pub fn hash_token(token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    hex::encode(hasher.finalize())
}
