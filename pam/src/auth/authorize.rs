use std::ffi::CStr;

use pam::{constants::PamResultCode, module::PamHandle};

use authentik_sys::generated::grpc_request;
use authentik_sys::generated::pam::SudoAuthorizationRequest;
use authentik_sys::generated::pam::pam_client::PamClient;

use crate::auth::interactive::result_to_pam_result;

pub fn authenticate_authorize_impl(
    _pamh: &mut PamHandle,
    _args: Vec<&CStr>,
    service: &str,
) -> PamResultCode {
    match grpc_request(async |ch| {
        return Ok(PamClient::new(ch)
            .sudo_authorize(SudoAuthorizationRequest {
                service: service.to_string(),
            })
            .await?);
    }) {
        Ok(r) => {
            let res = r.into_inner();
            result_to_pam_result(res.result)
        }
        Err(e) => {
            log::warn!("Failed to get groups: {e}");
            PamResultCode::PAM_PERM_DENIED
        }
    }
}
