use authentik_sys::generated::agent_auth::AuthorizeRequest;
use authentik_sys::generated::sys_auth::SystemAuthorizeRequest;
use authentik_sys::generated::sys_auth::system_auth_authorize_client::SystemAuthAuthorizeClient;
use authentik_sys::generated::{agent::RequestHeader, grpc_request};
use gethostname::gethostname;
use pam::{constants::PamResultCode, module::PamHandle};
use std::ffi::CStr;
use whoami::username;

use crate::ENV_SESSION_ID;
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
            return PamResultCode::PAM_IGNORE;
        }
    };
    let user = username();
    let ak = std::env::vars().find(|k| k.0 == ENV_SESSION_ID);
    let session_id = match ak {
        Some(s) => s.1,
        None => {
            log::warn!("Couldn't find session ID");
            return PamResultCode::PAM_IGNORE;
        }
    };
    match grpc_request(async |ch| {
        return Ok(SystemAuthAuthorizeClient::new(ch)
            .authorize(SystemAuthorizeRequest {
                session_id: session_id.clone(),
                authz: Some(AuthorizeRequest {
                    header: Some(RequestHeader {
                        profile: "".to_string(),
                    }),
                    uid: format!("pam-{host}-{user}-{service}-"),
                    service: service.to_string(),
                }),
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
