use crate::components::SysdContext;
use crate::util::to_status;
use ak_platform::generated::agent_auth::agent_auth_client::AgentAuthClient;
use ak_platform::generated::sys_auth::{
    InteractiveAuthResult, SystemAuthorizeRequest, SystemAuthorizeResponse,
};
use tonic::Status;

pub async fn authorize(
    ctx: &SysdContext,
    req: SystemAuthorizeRequest,
) -> Result<SystemAuthorizeResponse, Status> {
    let session = ctx
        .state
        .sessions()
        .get(&req.session_id)
        .await
        .map_err(to_status)?
        .ok_or_else(|| Status::not_found("session not found"))?;

    let local_socket = session
        .local_socket
        .ok_or_else(|| Status::failed_precondition("session has no local socket"))?;

    let authz_req = req
        .authz
        .ok_or_else(|| Status::invalid_argument("missing authz request"))?;

    let channel = ak_platform::grpc::grpc_endpoint(local_socket)
        .await
        .map_err(to_status)?;
    let mut client = AgentAuthClient::new(channel);
    let response = client
        .authorize(authz_req)
        .await
        .map_err(|e| Status::internal(format!("agent authorize failed: {e}")))?
        .into_inner();

    let successful = response.header.map(|h| h.successful).unwrap_or(false);
    Ok(SystemAuthorizeResponse {
        response: Some(response),
        code: if successful {
            InteractiveAuthResult::PamSuccess as i32
        } else {
            InteractiveAuthResult::PamPermDenied as i32
        },
    })
}
