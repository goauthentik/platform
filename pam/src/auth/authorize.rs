use authentik_sys::generated::{agent::RequestHeader, grpc_request_path};
use authentik_sys::generated::agent_auth::AuthorizeRequest;
use authentik_sys::generated::pam::pam_client::PamClient;
use gethostname::gethostname;
use pam::{constants::PamResultCode, module::PamHandle};
use std::ffi::CStr;
use whoami::username;

use crate::auth::interactive::result_to_pam_result;

pub fn authenticate_authorize_impl(
    _pamh: &mut PamHandle,
    _args: Vec<&CStr>,
    service: &str,
) -> PamResultCode {
    let binding = gethostname();
    let host = match binding.to_str() {
        Some(t) => t,
        None => {
            log::warn!("failed to get hostname");
            return PamResultCode::PAM_PERM_DENIED;
        }
    };
    let user = username();
    let cli_socket = match std::env::var("AUTHENTIK_CLI_SOCKET") {
        Ok(c) => c,
        Err(e) => {
            log::warn!("failed to get CLI socket path: {}", e);
            return PamResultCode::PAM_IGNORE;
        }
    };
    match grpc_request_path(cli_socket, async |ch| {
        return Ok(PamClient::new(ch)
            .authorize(AuthorizeRequest {
                header: Some(RequestHeader {
                    profile: "".to_string(),
                }),
                uid: format!("pam-{host}-{user}-{service}-"),
                service: service.to_string(),
            })
            .await?);
    }) {
        Ok(r) => {
            let res = r.into_inner();
            result_to_pam_result(res.code)
        }
        Err(e) => {
            log::warn!("Failed to authorize: {e}");
            PamResultCode::PAM_PERM_DENIED
        }
    }
}
