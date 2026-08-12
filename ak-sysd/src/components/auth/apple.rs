use crate::components::SysdContext;
use crate::util::to_status;
use ak_platform::generated::sys_auth_apple::register_device_response::BiometricPolicy;
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
    // TEMPORARY: authentik returns `biometric_policies` on this endpoint, but the
    // generated authentik-client has no field for it, so `res.biometric_policies`
    // does not compile against the pinned client. Hardcode the set the connector
    // is expected to send until the client is regenerated, then replace this with
    // the response value and delete the constant.
    //
    // PasswordFallback is included deliberately: without it a user whose Touch ID
    // is cancelled, failing, or never enrolled cannot use the key at all.
    let biometric_policies = vec![
        BiometricPolicy::TouchIdOrWatchCurrentSet as i32,
        BiometricPolicy::PasswordFallback as i32,
    ];
    tracing::info!(
        domain = %active.cfg.domain,
        biometric_policies = ?biometric_policies,
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
        biometric_policies,
        // Not part of the API response — the domain's own stored token,
        // mirroring Go's `device_token: dc.Token`.
        device_token: active.cfg.token.clone(),
    })
}
