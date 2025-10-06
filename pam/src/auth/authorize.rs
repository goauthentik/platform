use std::ffi::CStr;

use authentik_sys::generated::agent::RequestHeader;
use authentik_sys::generated::agent_auth::AuthorizeRequest;
use pam::{constants::PamResultCode, module::PamHandle};

use authentik_sys::generated::grpc_request;
use authentik_sys::generated::pam::pam_client::PamClient;

use crate::auth::interactive::result_to_pam_result;

pub fn authenticate_authorize_impl(
    _pamh: &mut PamHandle,
    _args: Vec<&CStr>,
    service: &str,
) -> PamResultCode {
    match grpc_request(async |ch| {
        return Ok(PamClient::new(ch)
            .authorize(AuthorizeRequest {
                header: Some(RequestHeader {
                    profile: "".to_string(),
                }),
                uid: format!("{service}-"),
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
