use crate::components::SysdContext;
use crate::util::to_status;
use ak_platform::generated::sys_auth_apple::{
    RegisterDeviceRequest, RegisterDeviceResponse, RegisterUserRequest, RegisterUserResponse,
    RegistrationStateRequest, RegistrationStateResponse, RegistrationStateUser,
    UnregisterDeviceRequest, UnregisterDeviceResponse,
};
use authentik_client::apis::Error;
use authentik_client::apis::endpoints_api::{
    endpoints_agents_psso_register_device_create, endpoints_agents_psso_register_device_destroy,
    endpoints_agents_psso_register_device_retrieve, endpoints_agents_psso_register_user_create,
};
use authentik_client::models::{
    AgentPssoDeviceRegistrationRequest, AgentPssoUserRegistrationRequest,
};
use eyre::{Result, bail};
use reqwest::StatusCode;
use tonic::Status;

/// Component name for the Platform SSO key/value state.
const PSSO_KV: &str = "psso";
/// Set when an unregister could not reach authentik, so the next checkin retries it.
const KEY_PENDING_UNREGISTER: &str = "pending_unregister";

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
    Ok(RegisterDeviceResponse {
        client_id: res.client_id,
        issuer: res.issuer,
        token_endpoint: res.token_endpoint,
        jwks_endpoint: res.jwks_endpoint,
        audience: res.audience,
        nonce_endpoint: res.nonce_endpoint,
        device_token: active.cfg.token.clone(),
        authorization_endpoint: res.authorization_endpoint,
    })
}

/// What a response status from the registration endpoints means for us.
#[derive(Debug, PartialEq, Eq)]
enum Outcome {
    Ok,
    /// authentik rejected our device token, so the registration really is gone.
    DeviceUnknown,
    /// This authentik does not have the endpoint. Note that 404 lands here and
    /// never in `DeviceUnknown`: an older server answers 404/405 for routes it
    /// does not serve, and reading that as "this device was deleted" would have
    /// every Mac ask for a repair registration it does not need.
    Unsupported,
    /// Anything else, worth trying again later.
    Retry,
}

fn classify(status: StatusCode) -> Outcome {
    if status.is_success() {
        Outcome::Ok
    } else if matches!(status, StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN) {
        Outcome::DeviceUnknown
    } else if matches!(
        status,
        StatusCode::NOT_FOUND | StatusCode::METHOD_NOT_ALLOWED
    ) {
        Outcome::Unsupported
    } else {
        Outcome::Retry
    }
}

pub async fn registration_state(
    ctx: &SysdContext,
    _req: RegistrationStateRequest,
) -> Result<RegistrationStateResponse, Status> {
    let active = ctx.domains.active().await.map_err(to_status)?;
    match endpoints_agents_psso_register_device_retrieve(&active.api).await {
        Ok(res) => Ok(RegistrationStateResponse {
            device_registered: res.device_registered,
            sign_key_id: res.sign_key_id,
            enc_key_id: res.enc_key_id,
            users: res
                .users
                .into_iter()
                .map(|u| RegistrationStateUser {
                    username: u.username,
                    enclave_key_id: u.enclave_key_id,
                })
                .collect(),
        }),
        Err(Error::ResponseError(content)) => match classify(content.status) {
            Outcome::DeviceUnknown => {
                tracing::info!(status = %content.status, "device unknown to authentik");
                Ok(RegistrationStateResponse::default())
            }
            Outcome::Unsupported => Err(Status::failed_precondition(format!(
                "authentik does not support psso registration state: {}",
                content.status
            ))),
            _ => Err(Status::unavailable(format!(
                "psso registration_state failed: {}",
                content.status
            ))),
        },
        Err(e) => Err(Status::unavailable(format!(
            "psso registration_state failed: {e}"
        ))),
    }
}

pub async fn unregister_device(
    ctx: &SysdContext,
    _req: UnregisterDeviceRequest,
) -> Result<UnregisterDeviceResponse, Status> {
    match try_unregister(ctx).await {
        Ok(completed) => Ok(UnregisterDeviceResponse { completed }),
        Err(e) => {
            // The configuration profile is already gone, so the extension will not
            // be invoked again: queue the call for the next checkin instead of
            // leaving authentik with a stale registration forever.
            tracing::warn!("psso unregister failed, queueing for next checkin: {e}");
            ctx.state
                .component_kv(PSSO_KV)
                .set(KEY_PENDING_UNREGISTER, "1")
                .await
                .map_err(to_status)?;
            Ok(UnregisterDeviceResponse { completed: false })
        }
    }
}

/// `Ok(false)` means there is nothing more to try, as opposed to `Err`, which
/// asks the caller to queue the unregister for a later attempt.
async fn try_unregister(ctx: &SysdContext) -> Result<bool> {
    let active = ctx.domains.active().await?;
    let status = match endpoints_agents_psso_register_device_destroy(&active.api).await {
        Ok(()) => return Ok(true),
        Err(Error::ResponseError(content)) => content.status,
        Err(e) => bail!("psso unregister failed: {e}"),
    };
    match classify(status) {
        Outcome::Ok => Ok(true),
        // Retrying against an authentik without the endpoint, or with a token it
        // no longer accepts, only produces the same answer every checkin.
        Outcome::Unsupported => {
            tracing::warn!(%status, "authentik does not support psso unregister");
            Ok(false)
        }
        Outcome::DeviceUnknown => {
            tracing::info!(%status, "authentik already forgot this device");
            Ok(true)
        }
        Outcome::Retry => bail!("psso unregister failed: {status}"),
    }
}

/// Retries an unregister that could not reach authentik when it was requested.
/// Called from the periodic device checkin, the only thing still running once
/// the configuration profile is gone.
pub async fn drain_pending_unregister(ctx: &SysdContext) {
    let kv = ctx.state.component_kv(PSSO_KV);
    match kv.get(KEY_PENDING_UNREGISTER).await {
        // The store has no delete, so a drained entry is left as an empty value.
        Ok(Some(v)) if v == "1" => {}
        Ok(_) => return,
        Err(e) => {
            tracing::warn!("failed to read pending psso unregister: {e:?}");
            return;
        }
    }
    match try_unregister(ctx).await {
        // Either it went through or nothing more can be tried, both of which end
        // the retries rather than leaving the entry to fire on every checkin.
        Ok(completed) => {
            tracing::info!(completed, "drained pending psso unregister");
            if let Err(e) = kv.set(KEY_PENDING_UNREGISTER, "").await {
                tracing::warn!("failed to clear pending psso unregister: {e:?}");
            }
        }
        Err(e) => tracing::warn!("pending psso unregister still failing: {e:?}"),
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::context::testutils::test_context;

    /// An authentik without these endpoints answers 404 or 405, which must never
    /// read as "this device was deleted" -- that would have every Mac talking to
    /// an older server ask for a repair registration it does not need.
    #[test]
    fn test_missing_endpoint_is_not_a_missing_device() {
        assert_eq!(classify(StatusCode::NOT_FOUND), Outcome::Unsupported);
        assert_eq!(
            classify(StatusCode::METHOD_NOT_ALLOWED),
            Outcome::Unsupported
        );
        assert_eq!(classify(StatusCode::UNAUTHORIZED), Outcome::DeviceUnknown);
        assert_eq!(classify(StatusCode::FORBIDDEN), Outcome::DeviceUnknown);
        assert_eq!(classify(StatusCode::INTERNAL_SERVER_ERROR), Outcome::Retry);
        assert_eq!(classify(StatusCode::BAD_GATEWAY), Outcome::Retry);
        assert_eq!(classify(StatusCode::OK), Outcome::Ok);
        assert_eq!(classify(StatusCode::NO_CONTENT), Outcome::Ok);
    }

    /// With no reachable domain the unregister must be queued rather than dropped:
    /// once the configuration profile is gone the extension never runs again, so
    /// the checkin drain is the only thing left that can finish it.
    #[tokio::test]
    async fn test_unregister_queues_when_unreachable() {
        let ctx = test_context().await;
        let res = unregister_device(&ctx, UnregisterDeviceRequest {})
            .await
            .expect("unregister should report failure, not error");
        assert!(!res.completed);
        assert_eq!(
            ctx.state
                .component_kv(PSSO_KV)
                .get(KEY_PENDING_UNREGISTER)
                .await
                .expect("kv read"),
            Some("1".to_string()),
        );

        // Still unreachable, so the queued call survives for the next checkin.
        drain_pending_unregister(&ctx).await;
        assert_eq!(
            ctx.state
                .component_kv(PSSO_KV)
                .get(KEY_PENDING_UNREGISTER)
                .await
                .expect("kv read"),
            Some("1".to_string()),
        );
    }
}
