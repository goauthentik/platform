use authentik_client::apis::Error;
use authentik_client::apis::configuration::Configuration;
use authentik_client::apis::flows_api::{
    FlowsExecutorGetError, FlowsExecutorSolveError, flows_executor_get, flows_executor_solve,
};
use authentik_client::models::{ChallengeTypes, ErrorDetail, FlowChallengeResponseRequest};
use cookie_store::Cookie;
use eyre::Report;
use reqwest::Client;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use reqwest_cookie_store::{CookieStore, CookieStoreMutex};
use std::collections::HashMap;
use std::fmt::Display;
use std::sync::Arc;

use crate::builder::FlowExecutorBuilder;
use crate::solvers::{
    AuthenticatorValidateSolver, IdentificationSolver, PasswordSolver, UserLoginSolver,
};

pub const HEADER_AUTHENTIK_REMOTE_IP: &str = "X-authentik-remote-ip";
pub const HEADER_AUTHENTIK_OUTPOST_TOKEN: &str = "X-authentik-outpost-token";

#[derive(Debug)]
pub enum FlowError {
    FlowGet(Error<FlowsExecutorGetError>),
    FlowSolve(Error<FlowsExecutorSolveError>),
    Other(Report),
}

impl Display for FlowError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FlowError::FlowGet(error) => error.fmt(f),
            FlowError::FlowSolve(error) => error.fmt(f),
            FlowError::Other(report) => report.fmt(f),
        }
    }
}
impl std::error::Error for FlowError {}

pub trait Solver {
    fn component(&self) -> String;
    fn solve(
        &self,
        ct: ChallengeTypes,
        answers: HashMap<String, String>,
    ) -> Result<FlowChallengeResponseRequest, FlowError>;
}

pub struct FlowExecutor {
    flow_slug: String,
    api_config: Configuration,
    nc: Option<ChallengeTypes>,

    pub(crate) solvers: Vec<Box<dyn Solver>>,
    pub(crate) answers: HashMap<String, String>,

    jar: Arc<CookieStoreMutex>,
}

impl FlowExecutor {
    pub fn builder() -> FlowExecutorBuilder {
        FlowExecutorBuilder::default()
    }

    pub(crate) async fn new(
        flow_slug: String,
        ref_config: Configuration,
        headers: HeaderMap,
    ) -> Result<Self, FlowError> {
        let jar = Arc::new(CookieStoreMutex::new(CookieStore::new()));
        let mut cfg = Configuration::new();
        cfg.user_agent = Some(ak_meta::user_agent());
        cfg.base_path = ref_config.base_path.clone();
        let mut hheaders = headers.clone();
        if let Some(at) = ref_config.bearer_access_token {
            hheaders.insert(
                HeaderName::from_static(HEADER_AUTHENTIK_OUTPOST_TOKEN),
                HeaderValue::from_str(&format!("Bearer {}", at))
                    .map_err(|e| FlowError::Other(eyre::eyre!(e)))?,
            );
        }
        cfg.client = reqwest_middleware::ClientBuilder::new(
            Client::builder()
                .cookie_provider(Arc::clone(&jar))
                .cookie_store(true)
                .build()
                .map_err(|e| FlowError::Other(eyre::eyre!(e)))?,
        )
        .build();
        Ok(Self {
            flow_slug,
            api_config: cfg,
            nc: None,
            solvers: vec![
                Box::new(IdentificationSolver {}),
                Box::new(PasswordSolver {}),
                Box::new(UserLoginSolver {}),
                Box::new(AuthenticatorValidateSolver {}),
            ],
            answers: HashMap::new(),
            jar,
        })
    }

    pub fn challenge(self) -> Option<ChallengeTypes> {
        self.nc.clone()
    }

    pub fn get_session(self) -> Result<Cookie<'static>, FlowError> {
        let url = url::Url::parse(&self.api_config.base_path)
            .map_err(|e| FlowError::Other(eyre::eyre!(e)))?;
        let Some(domain) = url.domain() else {
            return Err(FlowError::Other(eyre::eyre!("Failed to get domain")));
        };
        let jar = Arc::clone(&self.jar);
        let binding = jar
            .lock()
            .map_err(|e| FlowError::Other(eyre::eyre!("Failed to lock cookies: {:?}", e)))?;
        let cookie = binding.get_any(domain, url.path(), "authentik_session");
        if let Some(c) = cookie {
            return Ok(c.clone());
        }
        Err(FlowError::Other(eyre::eyre!("No cookie found")))
    }

    pub fn get_client(self) -> Client {
        self.api_config.client.as_ref().clone()
    }

    pub async fn start(&mut self) -> Result<(), FlowError> {
        let challenge = self.get_initial_challenge().await?;
        self.nc = Some(challenge);
        Ok(())
    }

    pub async fn execute(&mut self) -> Result<bool, FlowError> {
        tracing::trace!("Starting flow execution");
        self.start().await?;
        return self.solver(1).await;
    }

    async fn solver(&mut self, depth: i16) -> Result<bool, FlowError> {
        if depth >= 10 {
            return Err(FlowError::Other(eyre::eyre!(
                "exceeded stage recursion depth"
            )));
        }
        tracing::trace!(iteration = depth, "Solver iter");
        let done = self.solve_flow_challenge(None).await?;
        if done {
            return Ok(done);
        }
        return Box::pin(self.solver(depth + 1)).await;
    }

    pub async fn get_initial_challenge(&self) -> Result<ChallengeTypes, FlowError> {
        let challenge = flows_executor_get(&self.api_config, &self.flow_slug, "")
            .await
            .map_err(FlowError::FlowGet)?;
        tracing::trace!(component = challenge.component(), "Got initial challenge");
        Ok(challenge)
    }

    pub async fn solve_flow_challenge(
        &mut self,
        response: Option<FlowChallengeResponseRequest>,
    ) -> Result<bool, FlowError> {
        let Some(nc) = self.nc.clone() else {
            return Err(FlowError::Other(eyre::eyre!("no current stage")));
        };
        let mut res = response;
        if res.is_none() {
            if let Some(errors) = nc.response_errors() {
                for (key, errs) in errors {
                    if let Some(err) = errs.into_iter().next() {
                        return Err(FlowError::Other(eyre::eyre!(
                            "flow error {}: {}",
                            key,
                            err.string
                        )));
                    }
                }
            }
            match nc {
                ChallengeTypes::AkStageAccessDenied(_) => {
                    tracing::warn!("access denied");
                    return Ok(false);
                }
                ChallengeTypes::XakFlowRedirect(_) => {
                    tracing::info!("Flow finished");
                    return Ok(true);
                }
                ChallengeTypes::AkProviderOauth2DeviceCodeFinish(_) => {
                    tracing::info!("Flow finished");
                    return Ok(true);
                }
                _ => {
                    let component = nc.component();
                    tracing::trace!(component, "Finding solver for component");
                    for solver in &self.solvers {
                        if solver.component() != component {
                            continue;
                        }
                        tracing::trace!(component, "Found solver for component");
                        res = Some(solver.solve(nc.clone(), self.answers.clone())?);
                    }
                }
            };
        }
        tracing::trace!("Got response for challenge {:?}", res);

        let challenge = flows_executor_solve(&self.api_config, &self.flow_slug, "", res)
            .await
            .map_err(FlowError::FlowSolve)?;
        tracing::trace!(component = challenge.component(), "Got challenge");
        self.nc = Some(challenge.clone());
        Ok(false)
    }
}

trait ChallengeCommon {
    fn component(&self) -> String;
    fn response_errors(&self) -> Option<HashMap<String, Vec<ErrorDetail>>>;
}

impl ChallengeCommon for ChallengeTypes {
    fn component(&self) -> String {
        match &self {
            ChallengeTypes::AkStageAccessDenied(_) => "ak-stage-access-denied".to_string(),
            ChallengeTypes::AkSourceOauthApple(_) => "ak-source-oauth-apple".to_string(),
            ChallengeTypes::AkStageAuthenticatorDuo(_) => "ak-stage-authenticator-duo".to_string(),
            ChallengeTypes::AkStageAuthenticatorEmail(_) => {
                "ak-stage-authenticator-email".to_string()
            }
            ChallengeTypes::AkStageAuthenticatorSms(_) => "ak-stage-authenticator-sms".to_string(),
            ChallengeTypes::AkStageAuthenticatorStatic(_) => {
                "ak-stage-authenticator-static".to_string()
            }
            ChallengeTypes::AkStageAuthenticatorTotp(_) => {
                "ak-stage-authenticator-totp".to_string()
            }
            ChallengeTypes::AkStageAuthenticatorValidate(_) => {
                "ak-stage-authenticator-validate".to_string()
            }
            ChallengeTypes::AkStageAuthenticatorWebauthn(_) => {
                "ak-stage-authenticator-webauthn".to_string()
            }
            ChallengeTypes::AkStageAutosubmit(_) => "ak-stage-autosubmit".to_string(),
            ChallengeTypes::AkStageCaptcha(_) => "ak-stage-captcha".to_string(),
            ChallengeTypes::AkStageConsent(_) => "ak-stage-consent".to_string(),
            ChallengeTypes::AkStageDummy(_) => "ak-stage-dummy".to_string(),
            ChallengeTypes::AkStageEmail(_) => "ak-stage-email".to_string(),
            ChallengeTypes::AkStageEndpointAgent(_) => "ak-stage-endpoint-agent".to_string(),
            ChallengeTypes::AkStageFlowError(_) => "ak-stage-flow-error".to_string(),
            ChallengeTypes::XakFlowFrame(_) => "xak-flow-frame".to_string(),
            ChallengeTypes::AkStageIdentification(_) => "ak-stage-identification".to_string(),
            ChallengeTypes::AkProviderIframeLogout(_) => "ak-provider-iframe-logout".to_string(),
            ChallengeTypes::AkProviderSamlNativeLogout(_) => {
                "ak-provider-saml-native-logout".to_string()
            }
            ChallengeTypes::AkProviderOauth2DeviceCode(_) => {
                "ak-provider-oauth2-device-code".to_string()
            }
            ChallengeTypes::AkProviderOauth2DeviceCodeFinish(_) => {
                "ak-provider-oauth2-device-code-finish".to_string()
            }
            ChallengeTypes::AkStagePassword(_) => "ak-stage-password".to_string(),
            ChallengeTypes::AkSourcePlex(_) => "ak-source-plex".to_string(),
            ChallengeTypes::AkStagePrompt(_) => "ak-stage-prompt".to_string(),
            ChallengeTypes::XakFlowRedirect(_) => "xak-flow-redirect".to_string(),
            ChallengeTypes::AkStageSessionEnd(_) => "ak-stage-session-end".to_string(),
            ChallengeTypes::XakFlowShell(_) => "xak-flow-shell".to_string(),
            ChallengeTypes::AkSourceTelegram(_) => "ak-source-telegram".to_string(),
            ChallengeTypes::AkStageUserLogin(_) => "ak-stage-user-login".to_string(),
        }
    }
    fn response_errors(&self) -> Option<HashMap<String, Vec<ErrorDetail>>> {
        match self {
            ChallengeTypes::AkStageAccessDenied(access_denied_challenge) => {
                access_denied_challenge.response_errors.clone()
            }
            ChallengeTypes::AkSourceOauthApple(apple_login_challenge) => {
                apple_login_challenge.response_errors.clone()
            }
            ChallengeTypes::AkStageAuthenticatorDuo(authenticator_duo_challenge) => {
                authenticator_duo_challenge.response_errors.clone()
            }
            ChallengeTypes::AkStageAuthenticatorEmail(authenticator_email_challenge) => {
                authenticator_email_challenge.response_errors.clone()
            }
            ChallengeTypes::AkStageAuthenticatorSms(authenticator_sms_challenge) => {
                authenticator_sms_challenge.response_errors.clone()
            }
            ChallengeTypes::AkStageAuthenticatorStatic(authenticator_static_challenge) => {
                authenticator_static_challenge.response_errors.clone()
            }
            ChallengeTypes::AkStageAuthenticatorTotp(authenticator_totp_challenge) => {
                authenticator_totp_challenge.response_errors.clone()
            }
            ChallengeTypes::AkStageAuthenticatorValidate(authenticator_validation_challenge) => {
                authenticator_validation_challenge.response_errors.clone()
            }
            ChallengeTypes::AkStageAuthenticatorWebauthn(authenticator_web_authn_challenge) => {
                authenticator_web_authn_challenge.response_errors.clone()
            }
            ChallengeTypes::AkStageAutosubmit(autosubmit_challenge) => {
                autosubmit_challenge.response_errors.clone()
            }
            ChallengeTypes::AkStageCaptcha(captcha_challenge) => {
                captcha_challenge.response_errors.clone()
            }
            ChallengeTypes::AkStageConsent(consent_challenge) => {
                consent_challenge.response_errors.clone()
            }
            ChallengeTypes::AkStageDummy(dummy_challenge) => {
                dummy_challenge.response_errors.clone()
            }
            ChallengeTypes::AkStageEmail(email_challenge) => {
                email_challenge.response_errors.clone()
            }
            ChallengeTypes::AkStageEndpointAgent(endpoint_agent_challenge) => {
                endpoint_agent_challenge.response_errors.clone()
            }
            ChallengeTypes::AkStageFlowError(flow_error_challenge) => {
                flow_error_challenge.response_errors.clone()
            }
            ChallengeTypes::XakFlowFrame(frame_challenge) => {
                frame_challenge.response_errors.clone()
            }
            ChallengeTypes::AkStageIdentification(identification_challenge) => {
                identification_challenge.response_errors.clone()
            }
            ChallengeTypes::AkProviderIframeLogout(iframe_logout_challenge) => {
                iframe_logout_challenge.response_errors.clone()
            }
            ChallengeTypes::AkProviderSamlNativeLogout(native_logout_challenge) => {
                native_logout_challenge.response_errors.clone()
            }
            ChallengeTypes::AkProviderOauth2DeviceCode(oauth_device_code_challenge) => {
                oauth_device_code_challenge.response_errors.clone()
            }
            ChallengeTypes::AkProviderOauth2DeviceCodeFinish(
                oauth_device_code_finish_challenge,
            ) => oauth_device_code_finish_challenge.response_errors.clone(),
            ChallengeTypes::AkStagePassword(password_challenge) => {
                password_challenge.response_errors.clone()
            }
            ChallengeTypes::AkSourcePlex(plex_authentication_challenge) => {
                plex_authentication_challenge.response_errors.clone()
            }
            ChallengeTypes::AkStagePrompt(prompt_challenge) => {
                prompt_challenge.response_errors.clone()
            }
            ChallengeTypes::XakFlowRedirect(redirect_challenge) => {
                redirect_challenge.response_errors.clone()
            }
            ChallengeTypes::AkStageSessionEnd(session_end_challenge) => {
                session_end_challenge.response_errors.clone()
            }
            ChallengeTypes::XakFlowShell(shell_challenge) => {
                shell_challenge.response_errors.clone()
            }
            ChallengeTypes::AkSourceTelegram(telegram_login_challenge) => {
                telegram_login_challenge.response_errors.clone()
            }
            ChallengeTypes::AkStageUserLogin(user_login_challenge) => {
                user_login_challenge.response_errors.clone()
            }
        }
    }
}
