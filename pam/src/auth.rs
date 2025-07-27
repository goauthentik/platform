use std::{ffi::CStr, fs::File, io::Write, os::unix::fs::PermissionsExt};
use rand::Rng;

use pam::{constants::{PamFlag, PamResultCode, PAM_PROMPT_ECHO_OFF}, conv::Conv, module::PamHandle, pam_try};

use crate::{auth::{interactive::auth_interactive, token::auth_token}, config::Config, pam_env::pam_put_env, session::SessionData};

pub mod interactive;
pub mod token;

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
        Some(password) => Some(pam_try!(password.to_str(), PamResultCode::PAM_AUTH_ERR)),
        None => {
            unreachable!("No password");
        }
    };

    let id =_generate_id().to_string();
    let session_data = SessionData{
        username: username.to_string(),
        token: password.unwrap().to_owned(),
    };
    pam_try!(pam_put_env(pamh, "AUTHENTIK_SESSION_ID", id.to_owned().as_str()));
    pam_try!(_write_session_data(id, session_data));

    if password.unwrap_or("").starts_with("\u{200b}") ||
        password.unwrap_or("").starts_with("ey") {
        log::debug!("Password has token marker");
        let token = password.unwrap().replace("\u{200b}", "");
        // pam_try!(pamh.set_data("token", Box::new(token.to_owned())));
        return auth_token(config, username, token);
    } else {
        log::debug!("Interactive authentication");
        // pam_try!(pamh.set_data("token", Box::new(password.to_owned())));
        return auth_interactive(username, password.unwrap(), &conv);
    }
}

pub fn _read_session_data(id: String) -> Result<SessionData, PamResultCode> {
    let path  = format!("/tmp/aksm-{}", id);
    let file = File::open(path).expect("Could not create file!");

    return match serde_json::from_reader(file) {
        Ok(t) => Ok(t),
        Err(e) => {
            log::warn!("failed to write session data: {}", e);
            return Err(PamResultCode::PAM_AUTH_ERR)
        }
    }
}

pub fn _write_session_data(id: String, data: SessionData) -> Result<(), PamResultCode> {
    let json_data = serde_json::to_string(&data).unwrap();
    let path  = format!("/tmp/aksm-{}", id);
    let mut file = File::create(path).expect("Could not create file!");

    let mut permissions = file.metadata().unwrap().permissions();
    permissions.set_mode(permissions.mode() | 0o700);
    file.set_permissions(permissions);

    return match file.write_all(json_data.as_bytes()) {
        Ok(_) => Ok(()),
        Err(e) => {
            log::warn!("failed to write session data: {}", e);
            return Err(PamResultCode::PAM_AUTH_ERR)
        }
    }
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
