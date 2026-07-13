use std::{collections::HashMap, num::ParseIntError};

use authentik_client::models::{
    AuthenticatorValidationChallengeResponseRequest, ChallengeTypes, DeviceChallengeRequest,
    DeviceClassesEnum, FlowChallengeResponseRequest, IdentificationChallengeResponseRequest,
    PasswordChallengeResponseRequest, UserLoginChallengeResponseRequest,
};

use crate::{builder::FlowExecutorBuilder, executor::{FlowError, Solver}};
use regex::regex;
const CODE_PASSWORD_SEPARATOR: &str = ";";

#[derive(Default)]
pub struct IdentificationSolver;
impl Solver for IdentificationSolver {
    fn component(&self) -> String {
        "ak-stage-identification".to_string()
    }

    fn solve(
        &self,
        _ct: ChallengeTypes,
        answers: HashMap<String, String>,
    ) -> Result<FlowChallengeResponseRequest, FlowError> {
        let mut res = IdentificationChallengeResponseRequest::new();
        res.uid_field = Some(answers.get("ak-stage-identification").cloned());
        res.password = Some(answers.get("ak-stage-password").cloned());
        Ok(FlowChallengeResponseRequest::AkStageIdentification(res))
    }
}

#[derive(Default)]
pub struct PasswordSolver;
impl Solver for PasswordSolver {
    fn component(&self) -> String {
        "ak-stage-password".to_string()
    }

    fn solve(
        &self,
        _ct: ChallengeTypes,
        answers: HashMap<String, String>,
    ) -> Result<FlowChallengeResponseRequest, FlowError> {
        let Some(password) = answers.get("ak-stage-password") else {
            return Err(FlowError::Other(eyre::eyre!("No password answer given")));
        };
        let res = PasswordChallengeResponseRequest::new(password.clone());
        Ok(FlowChallengeResponseRequest::AkStagePassword(res))
    }
}

#[derive(Default)]
pub struct UserLoginSolver;
impl Solver for UserLoginSolver {
    fn component(&self) -> String {
        "ak-stage-user-login".to_string()
    }

    fn solve(
        &self,
        _ct: ChallengeTypes,
        _answers: HashMap<String, String>,
    ) -> Result<FlowChallengeResponseRequest, FlowError> {
        Ok(FlowChallengeResponseRequest::AkStageUserLogin(
            UserLoginChallengeResponseRequest::new(true),
        ))
    }
}

#[derive(Default)]
pub struct AuthenticatorValidateSolver;
impl Solver for AuthenticatorValidateSolver {
    fn component(&self) -> String {
        "ak-stage-authenticator-validate".to_string()
    }

    fn solve(
        &self,
        ct: ChallengeTypes,
        answers: HashMap<String, String>,
    ) -> Result<FlowChallengeResponseRequest, FlowError> {
        let mut res = AuthenticatorValidationChallengeResponseRequest::new();
        let ct = match ct {
            ChallengeTypes::AkStageAuthenticatorValidate(e) => e,
            _ => return Err(FlowError::Other(eyre::eyre!("Invalid challenge"))),
        };
        let mut matched = false;
        for dc in ct.device_challenges {
            if dc.device_class == DeviceClassesEnum::Duo {
                tracing::trace!(class = "duo", "selected device class");
                res.duo = Some(
                    dc.device_uid
                        .clone()
                        .parse()
                        .map_err(|e: ParseIntError| FlowError::Other(eyre::eyre!(e)))?,
                );
                res.selected_challenge = Some(DeviceChallengeRequest {
                    device_class: dc.device_class,
                    device_uid: dc.device_uid.clone(),
                    challenge: dc.challenge.clone(),
                    last_used: dc.last_used,
                });
                matched = true;
            }
            if dc.device_class == DeviceClassesEnum::Static
                || dc.device_class == DeviceClassesEnum::Totp
            {
                // Only use code-based devices if we have a code in the entered password,
                // and we haven't selected a push device yet
                if !matched && let Some(inp) = answers.get(&self.component()) {
                    tracing::trace!(class = "code", "selected device class");
                    res.selected_challenge = Some(DeviceChallengeRequest {
                        device_class: dc.device_class,
                        device_uid: dc.device_uid.clone(),
                        challenge: dc.challenge.clone(),
                        last_used: dc.last_used,
                    });
                    res.code = Some(inp.clone());
                    matched = true
                }
            }
        }
        if !matched {
            return Err(FlowError::Other(eyre::eyre!(
                "no compatible authenticator class found"
            )));
        }
        Ok(FlowChallengeResponseRequest::AkStageAuthenticatorValidate(
            res,
        ))
    }
}

impl FlowExecutorBuilder {
    pub fn set_secrets<T: ToString>(mut self, password: T, mfa_code_based: bool) -> Self {
        let comp_auth = AuthenticatorValidateSolver {}.component();
        let comp_pass = PasswordSolver {}.component();

        if self.answers.contains_key(&comp_auth) || self.answers.contains_key(&comp_pass) {
            return self;
        }
        let pw = password.to_string();
        self.answers.insert(comp_pass.clone(), pw.clone());
        if !mfa_code_based {
            // If code-based MFA is disabled StageAuthenticatorValidate answer is set to password.
            // This allows flows with a mfa stage only.
            self.answers.insert(comp_auth, pw.clone());
            return self;
        }
        // password doesn't contain the separator
        if !pw.contains(CODE_PASSWORD_SEPARATOR) {
            return self;
        }
        // password ends with the separator, so it won't contain an answer
        if pw.ends_with(CODE_PASSWORD_SEPARATOR) {
            return self;
        }
        let Some(idx) = pw.find(CODE_PASSWORD_SEPARATOR) else {
            return self;
        };
        let authenticator = &pw[idx + 1..];
        // Authenticator is either 6 chars (totp code) or 8 chars (long totp or static)
        if authenticator.len() == 6 {
            // authenticator answer isn't purely numerical, so won't be value
            if authenticator.parse::<i32>().is_err() {
                return self;
            }
        } else if authenticator.len() == 8 {
            let alpha_num = regex!("^[a-zA-Z0-9]*$");
            // 8 chars can be a long totp or static token, so it needs to be alphanumerical
            if !alpha_num.is_match(authenticator) {
                return self;
            }
        } else {
            // Any other length, doesn't contain an answer
            return self;
        }
        self.answers.insert(comp_pass, pw[..idx].to_string());
        self.answers.insert(comp_auth, authenticator.to_string());
        self
    }
}
