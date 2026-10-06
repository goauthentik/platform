use crate::components::auth::AuthComponent;
use crate::components::{Component, SysdContext};
use crate::util::to_status;
use ak_platform::generated::agent::{RequestHeader, ResponseHeader};
use ak_platform::generated::agent_auth::{
    CurrentTokenRequest, agent_auth_client::AgentAuthClient, current_token_request,
};
use ak_platform::generated::sys_platform::{
    PlatformEndpointRequest, PlatformEndpointResponse,
    system_platform_server::{SystemPlatform, SystemPlatformServer},
};
use ak_platform::net::server::creds::ProcCredentials;
use ak_platform::paths::SysdSocketID;
use ak_platform::shared::AuthentikClaims;
use authentik_client::models::{AgentConfig, DeviceFactsRequest};
use eyre::{OptionExt, Result, bail};
use jsonwebtoken::{
    Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, decode_header,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tonic::{Request, Response, Status};

const DEFAULT_REFRESH_INTERVAL_SECS: u64 = 30 * 60;

#[derive(Debug)]
pub struct DeviceComponent {
    ctx: SysdContext,
}

impl DeviceComponent {
    pub fn new(ctx: SysdContext) -> DeviceComponent {
        DeviceComponent { ctx }
    }

    /// Gathers device facts off the async runtime.
    ///
    /// `ak_platform_facts::gather` drives the in-process osquery engine over
    /// blocking C++ FFI and routinely takes several seconds (it runs ten
    /// tables, including the `processes`/`users`/`groups` ones), so calling it
    /// directly from a task stalls every other task on that worker thread.
    async fn gather_facts() -> Result<DeviceFactsRequest> {
        tokio::task::spawn_blocking(ak_platform_facts::gather)
            .await
            .map_err(|e| eyre::eyre!("gathering device facts failed: {e}"))
    }

    /// Runs one checkin cycle for a single domain by name. Exposed
    /// separately from the background loop so `ctrl`'s `domain_enroll` can
    /// trigger an immediate checkin right after enrollment.
    #[tracing::instrument(skip_all, fields(domain_name))]
    pub async fn checkin_domain(&self, domain_name: &str) -> Result<()> {
        tracing::info!("Checking in...");
        let domains = self.ctx.domains.domains().await;
        let Some(domain) = domains.iter().find(|d| d.cfg.domain == domain_name) else {
            bail!("domain not found: {domain_name}");
        };
        let facts = Self::gather_facts().await?;
        authentik_client::apis::endpoints_api::endpoints_agents_connectors_check_in_create(
            &domain.api,
            Some(facts),
        )
        .await
        .map_err(|e| eyre::eyre!("checkin failed: {e}"))?;
        Ok(())
    }

    /// Asks the caller's agent for its token and returns its verified claims. The agent
    /// only answers for callers running as its own user, so a caller can't get another
    /// user's identity by pointing sysd at their agent.
    async fn caller_claims(
        &self,
        caller: Option<ProcCredentials>,
        req: &PlatformEndpointRequest,
        remote: AgentConfig,
    ) -> Result<AuthentikClaims> {
        let caller = caller.ok_or_eyre("no peer credentials")?;
        let caller_pid = u32::try_from(caller.pid())?;
        ensure_callers_socket(&req.agent_socket, &caller)?;
        let channel = ak_platform::grpc::grpc_endpoint(req.agent_socket.clone()).await?;
        let token = AgentAuthClient::new(channel)
            .get_current_token(CurrentTokenRequest {
                header: Some(RequestHeader {
                    profile: req.profile.clone(),
                }),
                r#type: current_token_request::Type::Verified as i32,
                caller_pid,
            })
            .await?
            .into_inner();
        let auth = self
            .ctx
            .registry
            .get::<AuthComponent>("auth")
            .ok_or_eyre("no auth component")?;
        Ok(auth.validate_token(token.raw, Some(remote)).await?.claims)
    }
}

#[tonic::async_trait]
impl Component for DeviceComponent {
    fn id() -> &'static str {
        "device"
    }

    async fn start(&self) -> Result<()> {
        let ctx = self.ctx.clone();
        tokio::spawn(async move {
            loop {
                let domains = ctx.domains.domains().await;
                for d in domains {
                    let facts = match DeviceComponent::gather_facts().await {
                        Ok(facts) => facts,
                        Err(e) => {
                            tracing::warn!(domain = d.cfg.domain, "{e:?}");
                            continue;
                        }
                    };
                    if let Err(e) = authentik_client::apis::endpoints_api::endpoints_agents_connectors_check_in_create(&d.api, Some(facts)).await {
                        tracing::warn!(domain = d.cfg.domain, "checkin failed: {e:?}");
                    }
                }
                let jitter = rand::random::<u64>() % 30;
                let delay = ctx
                    .domains
                    .min_refresh_interval()
                    .await
                    .unwrap_or(DEFAULT_REFRESH_INTERVAL_SECS)
                    + jitter;
                tracing::info!(delay, "Waiting seconds before next checkin...");
                tokio::select! {
                    _ = tokio::time::sleep(std::time::Duration::from_secs(delay)) => {}
                    _ = ctx.cancel.cancelled() => return,
                }
            }
        });
        Ok(())
    }

    async fn stop(&self) -> Result<()> {
        Ok(())
    }

    fn register(self: Arc<Self>, socket: SysdSocketID, routes: &mut tonic::service::RoutesBuilder) {
        if matches!(socket, SysdSocketID::Default) {
            routes.add_service(SystemPlatformServer::from_arc(self));
        }
    }
}

/// The agent socket path comes from the caller, so only use the caller's own: on unix
/// one owned by the caller's user, on Windows the agent pipe of the caller's session.
fn ensure_callers_socket(path: &str, caller: &ProcCredentials) -> Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        let uid = caller.uid().ok_or_eyre("unknown caller uid")?;
        if std::fs::metadata(path)?.uid() != uid {
            bail!("agent socket is not owned by the caller");
        }
    }
    #[cfg(windows)]
    if path != ak_platform::paths::windows_agent_pipe(u32::try_from(caller.pid())?)? {
        bail!("agent pipe is not the one of the caller's session");
    }
    Ok(())
}

#[derive(Serialize, Deserialize)]
struct EndpointClaims {
    iss: String,
    aud: String,
    atc: String,
    iat: i64,
    exp: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    sub: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    jti: Option<String>,
}

#[tonic::async_trait]
impl SystemPlatform for DeviceComponent {
    async fn signed_endpoint_header(
        &self,
        request: Request<PlatformEndpointRequest>,
    ) -> Result<Response<PlatformEndpointResponse>, Status> {
        let caller = request.extensions().get::<ProcCredentials>().cloned();
        let req = request.into_inner();
        let domains = self.ctx.domains.domains().await;

        let header = decode_header(&req.challenge).map_err(to_status)?;
        let kid = header
            .kid
            .ok_or_else(|| Status::invalid_argument("challenge missing kid"))?;

        for d in &domains {
            let Some(remote) = d.remote.read().await.clone() else {
                continue;
            };
            let Some(jwks_challenge) = &remote.jwks_challenge else {
                continue;
            };
            let jwks_value = match serde_json::to_value(jwks_challenge) {
                Ok(v) => v,
                Err(_) => continue,
            };
            let jwks: jsonwebtoken::jwk::JwkSet = match serde_json::from_value(jwks_value) {
                Ok(v) => v,
                Err(_) => continue,
            };
            let Some(jwk) = jwks.find(&kid) else {
                continue;
            };
            let Ok(key) = DecodingKey::from_jwk(jwk) else {
                continue;
            };
            let mut validation = Validation::new(header.alg);
            validation.validate_aud = false;
            if decode::<serde_json::Value>(&req.challenge, &key, &validation).is_err() {
                continue;
            }

            let user = match self.caller_claims(caller.clone(), &req, remote).await {
                Ok(c) => Some(c),
                Err(e) => {
                    tracing::debug!("not adding user to endpoint header: {e:?}");
                    None
                }
            };
            let now = chrono::Utc::now().timestamp();
            let claims = EndpointClaims {
                iss: ak_platform_facts::serial().unwrap_or_default(),
                aud: "goauthentik.io/platform/endpoint".to_string(),
                atc: req.challenge.clone(),
                iat: now,
                exp: now + 5 * 60,
                sub: user.as_ref().and_then(|c| c.sub.clone()),
                jti: user.and_then(|c| c.jti),
            };
            let signed = jsonwebtoken::encode(
                &Header::new(Algorithm::HS512),
                &claims,
                &EncodingKey::from_secret(d.cfg.token.as_bytes()),
            )
            .map_err(to_status)?;

            return Ok(Response::new(PlatformEndpointResponse {
                header: Some(ResponseHeader { successful: true }),
                message: signed,
            }));
        }

        Err(Status::permission_denied(
            "challenge did not validate against any loaded domain",
        ))
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::ensure_callers_socket;
    use ak_platform::net::server::creds::ProcCredentials;
    use std::os::unix::fs::MetadataExt;

    #[test]
    fn callers_socket() {
        let file = tempfile::NamedTempFile::new().unwrap();
        let path = file.path().to_str().unwrap();
        let uid = std::fs::metadata(path).unwrap().uid();
        let caller = |uid| ProcCredentials::new(None).with_uid(uid);
        assert!(ensure_callers_socket(path, &caller(Some(uid))).is_ok());
        assert!(ensure_callers_socket(path, &caller(Some(uid + 1))).is_err());
        assert!(ensure_callers_socket(path, &caller(None)).is_err());
        assert!(ensure_callers_socket("/nonexistent/agent.sock", &caller(Some(uid))).is_err());
    }
}
