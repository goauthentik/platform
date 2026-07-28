use std::sync::Arc;

use ak_flow_executor::executor::FlowExecutor;
use ak_platform::generated::sys_auth::system_auth_token_server::SystemAuthToken;
use ak_platform::generated::sys_auth::{
    InteractiveAuthResult, InteractiveChallenge, TokenAuthRequest,
    interactive_challenge::PromptMeta,
};
use authentik_client::apis::endpoints_api::endpoints_agents_connectors_auth_ia_create;
use authentik_client::models::{
    ChallengeTypes, DeviceClassesEnum, FlowChallengeResponseRequest,
    IdentificationChallengeResponseRequest, PasswordChallengeResponseRequest,
    UserLoginChallengeResponseRequest,
};
use hex::encode as hex_encode;
use sha2::{Digest, Sha256};
use tonic::{Request, Status};

use super::{fido, redirect_scheme};
use crate::cfg::domain::LoadedDomain;
use crate::components::SysdContext;
use crate::components::auth::AuthComponent;
use crate::util::to_status;

const QS_TOKEN: &str = "ak-auth-ia-token";
const PASSWORD_PROMPT: &str = "authentik Password: ";

// Value of the X-Authentik-Platform-Auth-DTH header.
pub(super) fn device_token_hash(token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    hex_encode(hasher.finalize())
}

pub struct InteractiveAuthTransaction {
    pub id: String,
    pub result: Option<InteractiveAuthResult>,
    ctx: SysdContext,
    domain: Arc<LoadedDomain>,
    fex: FlowExecutor,
    username: String,
    // Init password, submitted once then cleared.
    password: Option<String>,
}

impl InteractiveAuthTransaction {
    pub async fn new(
        id: String,
        ctx: SysdContext,
        domain: Arc<LoadedDomain>,
        flow_slug: String,
        username: String,
        password: String,
    ) -> Result<Self, Status> {
        let mut fex = FlowExecutor::builder()
            .flow(flow_slug)
            .reference_config(domain.api.clone())
            .build()
            .await
            .map_err(|e| Status::internal(format!("failed to build flow executor: {e}")))?;
        fex.start()
            .await
            .map_err(|e| Status::internal(format!("failed to start flow: {e}")))?;
        Ok(Self {
            id,
            result: None,
            ctx,
            domain,
            fex,
            username,
            password: Some(password).filter(|p| !p.is_empty()),
        })
    }

    /// Advances the flow, auto-solving identification/password where possible,
    /// until it produces a challenge for the client (or finishes).
    #[tracing::instrument(fields(self.id), skip_all)]
    pub async fn get_next_challenge(&mut self) -> Result<InteractiveChallenge, Status> {
        let Some(ch) = self.fex.challenge() else {
            return Err(Status::internal("no current flow challenge"));
        };
        tracing::trace!(challenge = ?ch, "challenge");
        match ch {
            ChallengeTypes::XakFlowRedirect(_) => self.finish_success().await,
            ChallengeTypes::AkStageAccessDenied(c) => {
                self.result = Some(InteractiveAuthResult::PamPermDenied);
                Ok(InteractiveChallenge {
                    txid: self.id.clone(),
                    finished: true,
                    result: InteractiveAuthResult::PamPermDenied as i32,
                    prompt: c.error_message.unwrap_or_default(),
                    prompt_meta: PromptMeta::PamErrorMsg as i32,
                    component: "ak-stage-access-denied".to_string(),
                    ..Default::default()
                })
            }
            ChallengeTypes::AkStageIdentification(c) => {
                if !c.password_fields {
                    let username = self.username.clone();
                    if let Some(err) = self.solve_challenge(username).await? {
                        return Ok(err);
                    }
                    return Box::pin(self.get_next_challenge()).await;
                }
                // Combined password field: Go returns a bare challenge here.
                Ok(InteractiveChallenge {
                    txid: self.id.clone(),
                    ..Default::default()
                })
            }
            ChallengeTypes::AkStagePassword(_) => {
                if let Some(password) = self.password.take() {
                    if let Some(err) = self.solve_challenge(password).await? {
                        return Ok(err);
                    }
                    return Box::pin(self.get_next_challenge()).await;
                }
                Ok(InteractiveChallenge {
                    txid: self.id.clone(),
                    prompt: PASSWORD_PROMPT.to_string(),
                    prompt_meta: PromptMeta::PamPromptEchoOff as i32,
                    component: "ak-stage-password".to_string(),
                    ..Default::default()
                })
            }
            ChallengeTypes::AkStageAuthenticatorValidate(c) => {
                let dc = c
                    .device_challenges
                    .iter()
                    .find(|d| d.device_class == DeviceClassesEnum::Webauthn)
                    .ok_or_else(|| Status::internal("no webauthn device challenge available"))?;
                fido::parse_webauthn_request(&self.id, dc, &self.domain.cfg.authentik_url)
            }
            // "Empty challenge" per authentik's own docs — finalizes the session
            // after password validation and needs no user input, just an
            // auto-submitted response like identification/password above.
            ChallengeTypes::AkStageUserLogin(_) => {
                if let Some(err) = self.solve_challenge(String::new()).await? {
                    return Ok(err);
                }
                Box::pin(self.get_next_challenge()).await
            }
            _ => {
                tracing::warn!("unsupported interactive auth stage");
                Ok(InteractiveChallenge {
                    txid: self.id.clone(),
                    ..Default::default()
                })
            }
        }
    }

    /// Submits `value` for the current stage. `Ok(None)` on success;
    /// `Ok(Some(challenge))` carries a flow error back as an error prompt.
    #[tracing::instrument(fields(self.id), skip_all)]
    pub(super) async fn solve_challenge(
        &mut self,
        value: String,
    ) -> Result<Option<InteractiveChallenge>, Status> {
        let Some(ch) = self.fex.challenge() else {
            return Err(Status::internal("no current flow challenge"));
        };
        tracing::trace!(challenge = ?ch, "challenge");
        let req = match &ch {
            ChallengeTypes::AkStageIdentification(_) => {
                FlowChallengeResponseRequest::AkStageIdentification(
                    IdentificationChallengeResponseRequest {
                        uid_field: Some(Some(value)),
                        ..IdentificationChallengeResponseRequest::new()
                    },
                )
            }
            ChallengeTypes::AkStagePassword(_) => FlowChallengeResponseRequest::AkStagePassword(
                PasswordChallengeResponseRequest::new(value),
            ),
            ChallengeTypes::AkStageAuthenticatorValidate(c) => {
                let dc = c
                    .device_challenges
                    .iter()
                    .find(|d| d.device_class == DeviceClassesEnum::Webauthn)
                    .ok_or_else(|| Status::internal("no webauthn device challenge available"))?;
                fido::parse_webauthn_response(&value, dc, &self.domain.cfg.authentik_url)?
            }
            ChallengeTypes::AkStageUserLogin(_) => FlowChallengeResponseRequest::AkStageUserLogin(
                UserLoginChallengeResponseRequest::new(false),
            ),
            _ => return Err(Status::internal("cannot solve unsupported flow stage")),
        };
        match self.fex.solve_flow_challenge(Some(req)).await {
            Ok(_) => Ok(None),
            Err(e) => Ok(Some(InteractiveChallenge {
                txid: self.id.clone(),
                prompt: e.to_string(),
                prompt_meta: PromptMeta::PamErrorMsg as i32,
                ..Default::default()
            })),
        }
    }

    /// Exchanges the authenticated flow session for a one-time token via the
    /// finish redirect, then mints a real session from it.
    async fn finish_success(&mut self) -> Result<InteractiveChallenge, Status> {
        let ia = endpoints_agents_connectors_auth_ia_create(&self.domain.api)
            .await
            .map_err(|e| Status::internal(format!("failed to start interactive auth: {e}")))?;

        let dth = device_token_hash(&self.domain.cfg.token);
        let (client, captured) =
            redirect_scheme::build_finish_client(self.fex.cookie_jar()).map_err(to_status)?;

        // The token is captured from the redirect, not the response body.
        let _ = client
            .get(&ia.url)
            .header("X-Authentik-Platform-Auth-DTH", &dth)
            .send()
            .await;

        let final_url = captured
            .lock()
            .map_err(|_| Status::internal("failed to read captured redirect"))?
            .take()
            .ok_or_else(|| Status::internal("interactive auth did not reach finish redirect"))?;

        if final_url.host_str() != Some("platform") || final_url.path() != "/finished" {
            return Err(Status::internal(format!(
                "failed to extract code from final URL: {final_url}"
            )));
        }

        let token = final_url
            .query_pairs()
            .find(|(k, _)| k == QS_TOKEN)
            .map(|(_, v)| v.into_owned())
            .ok_or_else(|| Status::internal("finish redirect missing auth token"))?;

        let auth = AuthComponent::new(self.ctx.clone());
        let res = auth
            .token_auth(Request::new(TokenAuthRequest {
                username: self.username.clone(),
                token,
            }))
            .await?;

        self.result = Some(InteractiveAuthResult::PamSuccess);
        Ok(InteractiveChallenge {
            txid: self.id.clone(),
            finished: true,
            result: InteractiveAuthResult::PamSuccess as i32,
            session_id: res.into_inner().session_id,
            ..Default::default()
        })
    }
}
