use ak_platform::generated::agent::RequestHeader;
use ak_platform::generated::agent_auth::AuthorizeRequest;
use ak_platform::generated::sys_auth::SystemAuthorizeRequest;
use ak_platform::generated::sys_auth::system_auth_authorize_client::SystemAuthAuthorizeClient;
use ak_platform::grpc::grpc_request;
use eyre::{Context, Result};
use gethostname::gethostname;
use pam::constants::PamResultCode;
use whoami::username;

use crate::ENV_SESSION_ID;
use crate::auth::interactive::result_to_pam_result;
use crate::dir::check_user_exists;

pub fn authenticate_authorize_impl(service: &str) -> Result<PamResultCode> {
    let binding = gethostname();
    let host = match binding.to_str() {
        Some(t) => t,
        None => {
            tracing::warn!("failed to get hostname");
            return Ok(PamResultCode::PAM_IGNORE);
        }
    };
    let user = username().context("Couldn't get username")?;
    // Check if user actually exists in authentik
    check_user_exists(user.clone())?;
    let ak = std::env::vars().find(|k| k.0 == ENV_SESSION_ID);
    let session_id = match ak {
        Some(s) => s.1,
        None => {
            tracing::warn!("Couldn't find session ID");
            return Ok(PamResultCode::PAM_IGNORE);
        }
    };
    let res = grpc_request(async |ch| {
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
    })
    .context("Failed to authorize")?
    .into_inner();
    Ok(result_to_pam_result(res.code))
}
