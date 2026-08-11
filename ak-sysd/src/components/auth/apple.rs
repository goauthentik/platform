use crate::components::SysdContext;
use crate::util::to_status;
use ak_platform::generated::sys_auth_apple::{
    RegisterDeviceRequest, RegisterDeviceResponse, RegisterUserRequest, RegisterUserResponse,
};
use authentik_client::apis::endpoints_api::{
    endpoints_agents_psso_register_device_create, endpoints_agents_psso_register_user_create,
};
use authentik_client::models::{
    AgentPssoDeviceRegistrationRequest, AgentPssoUserRegistrationRequest,
};
use tonic::Status;

pub async fn register_user(
    ctx: &SysdContext,
    req: RegisterUserRequest,
) -> Result<RegisterUserResponse, Status> {
    let active = ctx.domains.active().await.map_err(to_status)?;
    let body = AgentPssoUserRegistrationRequest {
        user_auth: req.user_auth,
        user_secure_enclave_key: req.user_secure_enclave_key,
        enclave_key_id: req.enclave_key_id,
    };
    let res = endpoints_agents_psso_register_user_create(&active.api, body)
        .await
        .map_err(|e| Status::internal(format!("psso register_user failed: {e}")))?;
    Ok(RegisterUserResponse {
        username: res.username,
    })
}

pub async fn register_device(
    ctx: &SysdContext,
    req: RegisterDeviceRequest,
) -> Result<RegisterDeviceResponse, Status> {
    let active = ctx.domains.active().await.map_err(to_status)?;
    let body = AgentPssoDeviceRegistrationRequest {
        device_signing_key: req.device_signing_key,
        device_encryption_key: req.device_encryption_key,
        sign_key_id: req.sign_key_id,
        enc_key_id: req.enc_key_id,
    };
    let res = endpoints_agents_psso_register_device_create(&active.api, body)
        .await
        .map_err(|e| Status::internal(format!("psso register_device failed: {e}")))?;
    // `res.require_biometrics` can't be logged yet — the field isn't in the
    // generated authentik-client, so the hardcoded value below is what actually
    // reaches PSSO. Log that instead, and switch to the response value together
    // with the `require_biometrics` field below once the client has it.
    tracing::info!(
        domain = %active.cfg.domain,
        require_biometrics = true,
        client_id = %res.client_id,
        "psso register_device response"
    );
    Ok(RegisterDeviceResponse {
        client_id: res.client_id,
        issuer: res.issuer,
        token_endpoint: res.token_endpoint,
        jwks_endpoint: res.jwks_endpoint,
        audience: res.audience,
        nonce_endpoint: res.nonce_endpoint,
        // TEMPORARY: hardcoded for testing. The server-side field exists but is not
        // yet in a released authentik-client, so reading `res.require_biometrics`
        // does not compile against the pinned client. Swap back to
        // `res.require_biometrics` once the API change lands upstream.
        require_biometrics: true,
        // Not part of the API response — the domain's own stored token,
        // mirroring Go's `device_token: dc.Token`.
        device_token: active.cfg.token.clone(),
    })
}
