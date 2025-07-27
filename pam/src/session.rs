extern crate pam;

use crate::generated::pam_session::RegisterSessionRequest;
use crate::generated::pam_session::session_manager_client::SessionManagerClient;
use pam::constants::PamResultCode;
use pam::module::PamHandle;
use pam::{constants::PamFlag, pam_try};
use serde::{Deserialize, Serialize};
use tokio::net::UnixStream;
use std::env;
use std::ffi::{CStr, CString};
use tokio::runtime::Runtime;
use tonic::transport::{Channel, Endpoint, Uri};
use tower::service_fn;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SessionData {
    pub username: String,
    pub token: String,
}

pub fn open_session_impl(
    pamh: &mut PamHandle,
    _args: Vec<&CStr>,
    _flags: PamFlag,
) -> PamResultCode {
    log::debug!("{:#?}", env::var("qwerqerqewr"));

    log::debug!("foo");
        match unsafe { pamh.get_data::<String>("my_key") } {
            Ok(data) => {
                log::debug!("DEBUG: Retrieved data: {}", data);
            }
            Err(PamResultCode::PAM_NO_MODULE_DATA) => {
                log::debug!("DEBUG: No module data found");
            }
            Err(e) => {
                log::debug!("DEBUG: Other error: {:?}", e);
            }
        }
    log::debug!("{:#?}", unsafe {pamh.get_data::<SessionData>("data") });
    log::debug!("{:#?}", unsafe {pamh.get_data::<&&CString>("username") });
    log::debug!("{:#?}", unsafe {pamh.get_data::<&CStr>("username") });
    log::debug!("{:#?}", unsafe {pamh.get_data::<&&CStr>("username") });
    let username = match unsafe { pamh.get_data::<&CString>("username") } {
        Ok(t) => t.to_str(),
        Err(e) => {
            log::debug!{"Error: {:#?}", e};
            return e
        },
    }
    .unwrap();
    log::debug!("bar");
    let token = match unsafe { pamh.get_data::<&CString>("token") } {
        Ok(t) => t.to_str(),
        Err(e) => return e,
    }
    .unwrap();
    log::debug!("username: {}, token {}", username, token);
    // let expires_at: u64 = match unsafe { pamh.get_data::<&CStr>("expires_at") } {
    //     Ok(t) => t.to_str().unwrap().parse::<u64>().unwrap_or(0),
    //     Err(e) => return e,
    // };

    let token_hash = hash_token(&token);
    let pid = std::process::id();
    let ppid = std::os::unix::process::parent_id();

    let rt = match Runtime::new() {
        Ok(rt) => rt,
        Err(_) => return PamResultCode::PAM_SESSION_ERR,
    };

    let mut client = match rt.block_on(create_grpc_client()) {
        Ok(res) => res,
        Err(_) => return PamResultCode::PAM_SESSION_ERR,
    };

    let request = tonic::Request::new(RegisterSessionRequest {
        username: username.to_owned(),
        token_hash,
        expires_at: 0,
        pid,
        ppid,
    });

    let response = match rt.block_on(client.register_session(request)) {
        Ok(res) => res,
        Err(_) => return PamResultCode::PAM_SESSION_ERR,
    };
    let session_info = response.into_inner();

    if !session_info.success {
        log::warn!("failed to add session: {}", session_info.error.to_string());
        return PamResultCode::PAM_SESSION_ERR;
    }

    pam_try!(pamh.set_data::<&CStr>("session_id", Box::new(CString::new(session_info.session_id).unwrap().as_c_str())));
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

pub fn hash_token(token: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    hex::encode(hasher.finalize())
}
