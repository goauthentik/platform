extern crate pam;

use crate::ENV_SESSION_ID;
use crate::auth::_read_session_data;
use crate::generated::pam_session::RegisterSessionRequest;
use crate::generated::pam_session::session_manager_client::SessionManagerClient;
use crate::pam_env::pam_get_env;
use pam::constants::PamResultCode;
use pam::module::PamHandle;
use pam::{constants::PamFlag, pam_try};
use serde::{Deserialize, Serialize};
use std::ffi::{CStr, CString};
use tokio::net::UnixStream;
use tokio::runtime::Runtime;
use tonic::transport::{Channel, Endpoint, Uri};
use tower::service_fn;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SessionData {
    pub username: String,
    pub token: String,
    pub expiry: i64,
}

pub fn open_session_impl(
    pamh: &mut PamHandle,
    _args: Vec<&CStr>,
    _flags: PamFlag,
) -> PamResultCode {
    let id = pam_get_env(pamh, ENV_SESSION_ID).unwrap();

    let sd = pam_try!(_read_session_data(id.to_owned()));

    let pid = std::process::id();
    let ppid = std::os::unix::process::parent_id();

    let request = tonic::Request::new(RegisterSessionRequest {
        session_id: id,
        username: sd.username.to_owned(),
        token_hash: sd.token,
        expires_at: sd.expiry,
        pid,
        ppid,
    });

    let rt = match Runtime::new() {
        Ok(rt) => rt,
        Err(e) => {
            log::warn!("Failed to create runtime: {}", e);
            return PamResultCode::PAM_SESSION_ERR;
        }
    };
    let mut client = match rt.block_on(create_grpc_client()) {
        Ok(res) => res,
        Err(e) => {
            log::warn!("Failed to create grpc client: {}", e);
            return PamResultCode::PAM_SESSION_ERR;
        }
    };
    let response = match rt.block_on(client.register_session(request)) {
        Ok(res) => res,
        Err(e) => {
            log::warn!("failed to send GRPC request: {}", e);
            return PamResultCode::PAM_SESSION_ERR;
        }
    };
    let session_info = response.into_inner();

    if !session_info.success {
        log::warn!("failed to add session: {}", session_info.error.to_string());
        return PamResultCode::PAM_SESSION_ERR;
    }

    pam_try!(pamh.set_data::<&CStr>(
        "session_id",
        Box::new(CString::new(session_info.session_id).unwrap().as_c_str())
    ));
    PamResultCode::PAM_SUCCESS
}

async fn create_grpc_client() -> Result<SessionManagerClient<Channel>, Box<dyn std::error::Error>> {
    log::info!("creating grpc client");
    let channel = Endpoint::try_from("http://[::]:50051")?
        .connect_with_connector(service_fn(|_: Uri| {
            UnixStream::connect("/var/run/authentik-session-manager.sock")
        }))
        .await?;

    Ok(SessionManagerClient::new(channel))
}
