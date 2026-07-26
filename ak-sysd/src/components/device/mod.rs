use crate::components::{Component, SysdContext};
use crate::util::to_status;
use ak_platform::generated::agent::ResponseHeader;
use ak_platform::generated::agent_platform::{
    PlatformEndpointRequest, PlatformEndpointResponse,
    agent_platform_server::{AgentPlatform, AgentPlatformServer},
};
use ak_platform::paths::SysdSocketID;
use authentik_client::models::DeviceFactsRequest;
use eyre::{Result, bail};
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

    fn gather_facts() -> DeviceFactsRequest {
        ak_platform_facts::gather()
    }

    /// Runs one checkin cycle for a single domain by name. Exposed
    /// separately from the background loop so `ctrl`'s `domain_enroll` can
    /// trigger an immediate checkin right after enrollment.
    #[tracing::instrument]
    pub async fn checkin_domain(&self, domain_name: &str) -> Result<()> {
        tracing::info!("Checking in...");
        let domains = self.ctx.domains.domains().await;
        let Some(domain) = domains.iter().find(|d| d.cfg.domain == domain_name) else {
            bail!("domain not found: {domain_name}");
        };
        let facts = Self::gather_facts();
        authentik_client::apis::endpoints_api::endpoints_agents_connectors_check_in_create(
            &domain.api,
            Some(facts),
        )
        .await
        .map_err(|e| eyre::eyre!("checkin failed: {e}"))?;
        Ok(())
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
                    let jitter = rand::random::<u64>() % 30;
                    tokio::select! {
                        _ = tokio::time::sleep(std::time::Duration::from_secs(jitter)) => {}
                        _ = ctx.cancel.cancelled() => return,
                    }
                    let facts = DeviceComponent::gather_facts();
                    if let Err(e) = authentik_client::apis::endpoints_api::endpoints_agents_connectors_check_in_create(&d.api, Some(facts)).await {
                        tracing::warn!(domain = d.cfg.domain, "checkin failed: {e:?}");
                    }
                }
                tokio::select! {
                    _ = tokio::time::sleep(std::time::Duration::from_secs(DEFAULT_REFRESH_INTERVAL_SECS)) => {}
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
            routes.add_service(AgentPlatformServer::from_arc(self));
        }
    }
}

#[derive(Serialize, Deserialize)]
struct EndpointClaims {
    iss: String,
    aud: String,
    atc: String,
    iat: i64,
    exp: i64,
}

#[tonic::async_trait]
impl AgentPlatform for DeviceComponent {
    async fn signed_endpoint_header(
        &self,
        request: Request<PlatformEndpointRequest>,
    ) -> Result<Response<PlatformEndpointResponse>, Status> {
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

            let now = chrono::Utc::now().timestamp();
            let claims = EndpointClaims {
                iss: ak_platform_facts::serial().unwrap_or_default(),
                aud: "goauthentik.io/platform/endpoint".to_string(),
                atc: req.challenge.clone(),
                iat: now,
                exp: now + 5 * 60,
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
