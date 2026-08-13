use ak_platform::{
    generated::{
        agent::RequestHeader,
        agent_auth::AuthorizeRequest,
        sys_auth::{
            SystemAuthorizeRequest, system_auth_authorize_client::SystemAuthAuthorizeClient,
        },
    },
    grpc::grpc_request_tunnel,
};
use eyre::{Context, Result};
use pam::constants::PamResultCode;

use crate::auth::interactive::result_to_pam_result;

pub const SSH_AUTH_SOCK: &str = "SSH_AUTH_SOCK";

pub fn authenticate_authorize_ssh(
    sock_path: String,
    service: String,
    host: String,
    user: String,
    session_id: String,
) -> Result<PamResultCode> {
    let res = grpc_request_tunnel(sock_path, async |ch| {
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
