use ak_platform::generated::agent::RequestHeader;
use ak_platform::generated::agent_auth::AuthorizeRequest;
use ak_platform::generated::sys_auth::SystemAuthorizeRequest;
use ak_platform::generated::sys_auth::system_auth_authorize_client::SystemAuthAuthorizeClient;
use ak_platform::grpc::grpc_request;
use eyre::{Context, Result};
use gethostname::gethostname;
use pam::constants::PamResultCode;
use pam::module::PamHandle;

use crate::{ENV_SESSION_ID, username};
use crate::auth::interactive::result_to_pam_result;
use crate::auth::ssh::{SSH_AUTH_SOCK, authenticate_authorize_ssh};
use crate::dir::check_user_exists;

pub fn authenticate_authorize_impl(pamh: &mut PamHandle, service: String) -> Result<PamResultCode> {
    let binding = gethostname();
    let host = match binding.to_str() {
        Some(t) => t.to_string(),
        None => {
            tracing::warn!("failed to get hostname");
            return Ok(PamResultCode::PAM_IGNORE);
        }
    };
    let user = username(pamh).context("Couldn't get username")?;
    // Check if user actually exists in authentik
    check_user_exists(user.clone())?;
    let Some((_, session_id)) = std::env::vars().find(|k| k.0 == ENV_SESSION_ID) else {
        tracing::warn!("Couldn't find session ID");
        return Ok(PamResultCode::PAM_IGNORE);
    };

    if let Some((_, ssh)) = std::env::vars().find(|k| k.0 == SSH_AUTH_SOCK) {
        return authenticate_authorize_ssh(ssh, service, host, user, session_id);
    }

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
