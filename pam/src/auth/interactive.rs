use authentik_sys::generated::sys_auth::{
    InteractiveAuthContinueRequest, InteractiveAuthInitRequest, InteractiveAuthRequest,
    InteractiveAuthResult, InteractiveChallenge, interactive_auth_request::InteractiveAuth,
    interactive_challenge::PromptMeta, system_auth_interactive_client::SystemAuthInteractiveClient,
};
use authentik_sys::grpc::{SysdBridge, encode_pb};
use pam::{
    constants::{
        PAM_BINARY_PROMPT, PAM_ERROR_MSG, PAM_PROMPT_ECHO_OFF, PAM_PROMPT_ECHO_ON, PAM_RADIO_TYPE,
        PAM_TEXT_INFO, PamMessageStyle, PamResultCode,
    },
    conv::Conv,
};

use crate::auth::PW_PROMPT;
use crate::auth::fido::fido2;

const MAX_ITER: i8 = 30;

pub fn result_to_pam_result(result: i32) -> PamResultCode {
    match InteractiveAuthResult::try_from(result) {
        Ok(InteractiveAuthResult::PamSuccess) => PamResultCode::PAM_SUCCESS,
        Ok(InteractiveAuthResult::PamPermDenied) => PamResultCode::PAM_PERM_DENIED,
        Ok(InteractiveAuthResult::PamAuthErr) => PamResultCode::PAM_AUTH_ERR,
        Err(_) => PamResultCode::PAM_SYSTEM_ERR,
    }
}

pub fn prompt_meta_to_pam_message_style(challenge: &InteractiveChallenge) -> PamMessageStyle {
    match PromptMeta::try_from(challenge.prompt_meta) {
        Ok(PromptMeta::PamBinaryPrompt) => PAM_BINARY_PROMPT,
        Ok(PromptMeta::PamErrorMsg) => PAM_ERROR_MSG,
        Ok(PromptMeta::PamPromptEchoOff) => PAM_PROMPT_ECHO_OFF,
        Ok(PromptMeta::PamPromptEchoOn) => PAM_PROMPT_ECHO_ON,
        Ok(PromptMeta::PamRadioType) => PAM_RADIO_TYPE,
        Ok(PromptMeta::PamTextInfo) => PAM_TEXT_INFO,
        Ok(_) => PAM_PROMPT_ECHO_OFF,
        Err(_) => PAM_PROMPT_ECHO_OFF,
    }
}

pub fn auth_interactive(
    username: String,
    password: String,
    conv: &Conv<'_>,
    bridge: impl SysdBridge,
) -> Result<InteractiveChallenge, PamResultCode> {
    // Init transaction
    let mut challenge = match bridge.grpc_request(async |ch| {
        return Ok(SystemAuthInteractiveClient::new(ch)
            .interactive_auth(InteractiveAuthRequest {
                interactive_auth: Some(InteractiveAuth::Init(InteractiveAuthInitRequest {
                    username: username.to_owned(),
                    password: password.to_owned(),
                })),
            })
            .await?);
    }) {
        Ok(t) => t.into_inner(),
        Err(e) => {
            log::warn!("failed to init interactive auth: {e}");
            return Err(PamResultCode::PAM_AUTH_ERR);
        }
    };
    // We always prompt for password to distinguish between token/interactive
    // so at this point we've always statically prompted for password.
    // In case this initial prompt from the server fails, re-attempt the password challenge
    let mut prev_challenge = InteractiveChallenge {
        txid: challenge.txid.to_owned(),
        finished: false,
        result: 0,
        prompt: PW_PROMPT.to_owned(),
        prompt_meta: PAM_PROMPT_ECHO_OFF,
        debug_info: "".to_owned(),
        session_id: "".to_owned(),
        component: "".to_owned(),
    };
    let mut iter = -1;
    while iter <= MAX_ITER {
        iter += 1;
        log::debug!("{} processing challenge: {:?}", iter, challenge);
        if challenge.finished {
            if !challenge.prompt.is_empty() {
                match conv.send(
                    prompt_meta_to_pam_message_style(&challenge),
                    &challenge.prompt,
                ) {
                    Ok(_) => {}
                    Err(e) => {
                        log::warn!("failed to send prompt");
                        return Err(e);
                    }
                };
            }
            return Ok(challenge);
        }
        let mut req_inner = InteractiveAuthContinueRequest {
            txid: challenge.txid.to_owned(),
            value: "".to_owned(),
        };
        // Depending on the prompt, either prompt for data or use what we already know
        match PromptMeta::try_from(challenge.prompt_meta) {
            Ok(PromptMeta::Unspecified) => {
                log::warn!("Unspecified prompt meta");
                return Err(PamResultCode::PAM_ABORT);
            }
            Ok(PromptMeta::PamBinaryPrompt) => {
                match fido2(challenge.prompt.clone(), conv) {
                    Ok(r) => {
                        req_inner.value = match encode_pb(r) {
                            Ok(v) => v,
                            Err(e) => {
                                log::warn!("Failed to reply to WebAuthn: {}", e);
                                return Err(PamResultCode::PAM_ABORT);
                            }
                        };
                    }
                    Err(e) => {
                        log::warn!("Failed to Fido2 authenticate: {}", e);
                        continue;
                    }
                };
            }
            Ok(_) => {
                log::debug!("Prompt meta generic, prompt user");
                let style = prompt_meta_to_pam_message_style(&challenge);
                let credential = match conv.send(style, &challenge.prompt) {
                    Ok(c) => match c {
                        Some(c) => match c.to_str() {
                            Ok(cc) => cc,
                            Err(_) => {
                                log::warn!("failed to convert PAM Conversation response to string");
                                return Err(PamResultCode::PAM_ABORT);
                            }
                        },
                        None => {
                            if [PAM_ERROR_MSG, PAM_TEXT_INFO].contains(&style) {
                                challenge = prev_challenge.clone();
                                log::debug!("Restarting loop due to message");
                                continue;
                            }
                            log::warn!("No PAM conversation response");
                            return Err(PamResultCode::PAM_ABORT);
                        }
                    },
                    Err(e) => {
                        return Err(e);
                    }
                };
                req_inner.value = credential.to_owned();
            }
            Err(_) => {
                log::warn!(
                    "Failed to convert prompt meta value to allowed values: {}",
                    challenge.prompt_meta
                );
                return Err(PamResultCode::PAM_ABORT);
            }
        }
        prev_challenge = challenge;
        // Send the response
        challenge = match bridge.grpc_request(async |ch| {
            return Ok(SystemAuthInteractiveClient::new(ch)
                .interactive_auth(InteractiveAuthRequest {
                    interactive_auth: Some(InteractiveAuth::Continue(req_inner.to_owned())),
                })
                .await?);
        }) {
            Ok(t) => t.into_inner(),
            Err(e) => {
                log::warn!("failed to continue auth: {e}");
                return Err(PamResultCode::PAM_AUTH_ERR);
            }
        };
    }
    log::warn!("Exceeded maximum iterations");
    Err(PamResultCode::PAM_ABORT)
}

#[cfg(test)]
mod tests {
    use authentik_sys::generated::sys_auth::{
        InteractiveAuthResult, InteractiveChallenge, interactive_challenge::PromptMeta,
    };
    use pam::constants::{
        PAM_BINARY_PROMPT, PAM_ERROR_MSG, PAM_PROMPT_ECHO_OFF, PAM_PROMPT_ECHO_ON, PAM_RADIO_TYPE,
        PAM_TEXT_INFO, PamResultCode,
    };

    use super::{prompt_meta_to_pam_message_style, result_to_pam_result};

    fn challenge(prompt_meta: PromptMeta) -> InteractiveChallenge {
        InteractiveChallenge {
            txid: String::new(),
            finished: false,
            result: 0,
            prompt: String::new(),
            prompt_meta: prompt_meta as i32,
            debug_info: String::new(),
            session_id: String::new(),
            component: String::new(),
        }
    }

    #[test]
    fn maps_interactive_results_to_pam_codes() {
        assert_eq!(
            result_to_pam_result(InteractiveAuthResult::PamSuccess as i32),
            PamResultCode::PAM_SUCCESS
        );
        assert_eq!(
            result_to_pam_result(InteractiveAuthResult::PamPermDenied as i32),
            PamResultCode::PAM_PERM_DENIED
        );
        assert_eq!(
            result_to_pam_result(InteractiveAuthResult::PamAuthErr as i32),
            PamResultCode::PAM_AUTH_ERR
        );
    }

    #[test]
    fn falls_back_to_system_error_for_unknown_results() {
        assert_eq!(result_to_pam_result(999), PamResultCode::PAM_SYSTEM_ERR);
    }

    #[test]
    fn maps_prompt_meta_values_to_pam_styles() {
        assert_eq!(
            prompt_meta_to_pam_message_style(&challenge(PromptMeta::PamBinaryPrompt)),
            PAM_BINARY_PROMPT
        );
        assert_eq!(
            prompt_meta_to_pam_message_style(&challenge(PromptMeta::PamErrorMsg)),
            PAM_ERROR_MSG
        );
        assert_eq!(
            prompt_meta_to_pam_message_style(&challenge(PromptMeta::PamPromptEchoOff)),
            PAM_PROMPT_ECHO_OFF
        );
        assert_eq!(
            prompt_meta_to_pam_message_style(&challenge(PromptMeta::PamPromptEchoOn)),
            PAM_PROMPT_ECHO_ON
        );
        assert_eq!(
            prompt_meta_to_pam_message_style(&challenge(PromptMeta::PamRadioType)),
            PAM_RADIO_TYPE
        );
        assert_eq!(
            prompt_meta_to_pam_message_style(&challenge(PromptMeta::PamTextInfo)),
            PAM_TEXT_INFO
        );
    }

    #[test]
    fn defaults_to_echo_off_for_unknown_prompt_meta() {
        let mut unknown = challenge(PromptMeta::Unspecified);
        unknown.prompt_meta = 999;

        assert_eq!(
            prompt_meta_to_pam_message_style(&unknown),
            PAM_PROMPT_ECHO_OFF
        );
    }
}
