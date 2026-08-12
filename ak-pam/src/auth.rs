use ak_platform::grpc::Bridge;
use pam::{constants::PamResultCode, conv::Conv, module::PamHandle};

use crate::{
    ENV_SESSION_ID,
    auth::interactive::auth_interactive,
    dir::check_user_exists,
    pam_env::pam_put_env,
    session_data::{_write_session_data, SessionData},
    username,
};
use eyre::{Context, Result};

pub mod authorize;
pub mod fido;
pub mod interactive;

pub fn authenticate_impl(pamh: &mut PamHandle) -> Result<PamResultCode> {
    let username = username(pamh)?;
    tracing::debug!("got username: '{username}'");
    // Check if user actually exists in authentik
    check_user_exists(username.clone())?;
    let conv = match pamh.get_item::<Conv>() {
        Ok(Some(conv)) => conv,
        Ok(None) => {
            unreachable!("No conv available");
        }
        Err(err) => {
            tracing::debug!("Couldn't get pam_conv");
            return Ok(err);
        }
    };
    tracing::debug!("Started conv");

    let session_data = SessionData {
        username: username.to_string(),
        local_socket: "".to_owned(),
    };

    let bridge = match Bridge::new() {
        Ok(b) => b,
        Err(e) => {
            tracing::warn!("Failed to get runtime {}", e);
            return Ok(PamResultCode::PAM_ABORT);
        }
    };

    tracing::debug!("Interactive authentication");
    let int_res = match auth_interactive(username, &conv, bridge) {
        Ok(ss) => ss,
        Err(code) => return Ok(code),
    };
    let session_id: String = int_res.session_id;
    if !session_data.local_socket.is_empty() {
        pam_put_env(
            pamh,
            "AUTHENTIK_CLI_SOCKET",
            session_data.local_socket.to_owned().as_str(),
        )
        .context("Failed to set env")?;
    }
    _write_session_data(session_id.clone(), session_data)
        .context("failed to write session data")?;
    pam_put_env(pamh, ENV_SESSION_ID, session_id.to_owned().as_str())
        .context("failed to set session_id env")?;
    Ok(PamResultCode::PAM_SUCCESS)
}
