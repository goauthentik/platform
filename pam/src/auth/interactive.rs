use authentik_sys::generated::{
    grpc_request,
    pam::{
        InteractiveAuthContinueRequest, InteractiveAuthInitRequest, InteractiveAuthRequest,
        InteractiveAuthResult, InteractiveChallenge, interactive_auth_request::InteractiveAuth,
        interactive_challenge::PromptMeta, pam_client::PamClient,
    },
};
use pam::{
    constants::{
        PAM_BINARY_PROMPT, PAM_ERROR_MSG, PAM_PROMPT_ECHO_OFF, PAM_PROMPT_ECHO_ON, PAM_RADIO_TYPE,
        PAM_TEXT_INFO, PamMessageStyle, PamResultCode,
    },
    conv::Conv,
};

use crate::pam_try_log;

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

pub fn auth_interactive(username: String, password: String, conv: &Conv<'_>) -> PamResultCode {
    // Init transaction
    let mut challenge = match grpc_request(async |ch| {
        return Ok(PamClient::new(ch)
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
            return PamResultCode::PAM_AUTH_ERR;
        }
    };
    let mut prev_challenge = challenge.clone();
    let mut iter = 0;
    while iter <= MAX_ITER {
        if challenge.finished {
            return result_to_pam_result(challenge.result);
        }
        let mut req_inner = InteractiveAuthContinueRequest {
            txid: challenge.txid.to_owned(),
            value: "".to_owned(),
        };
        // Depending on the prompt, either prompt for data or use what we already know
        match PromptMeta::try_from(challenge.prompt_meta) {
            Ok(PromptMeta::Unspecified) => {
                log::warn!("Unspecified prompt meta");
                return PamResultCode::PAM_ABORT;
            }
            Ok(_) => {
                log::debug!("Prompt meta generic, prompt user");
                let style = prompt_meta_to_pam_message_style(&challenge);
                let credential = match pam_try_log!(
                    conv.send(style, &challenge.prompt),
                    "failed to send prompt"
                ) {
                    Some(c) => match c.to_str() {
                        Ok(cc) => cc,
                        Err(_) => {
                            log::warn!("failed to convert PAM Conversation response to string");
                            return PamResultCode::PAM_ABORT;
                        }
                    },
                    None => {
                        if [PAM_ERROR_MSG, PAM_TEXT_INFO].contains(&style) {
                            challenge = prev_challenge.clone();
                            log::debug!("Restarting loop due to message");
                            return PamResultCode::PAM_PERM_DENIED;
                        }
                        log::warn!("No PAM conversation response");
                        return PamResultCode::PAM_ABORT;
                    }
                };
                req_inner.value = credential.to_owned();
            }
            Err(_) => {
                log::warn!(
                    "Failed to convert prompt meta value to allowed values: {}",
                    challenge.prompt_meta
                );
                return PamResultCode::PAM_ABORT;
            }
        }
        prev_challenge = challenge;
        // Send the response
        challenge = match grpc_request(async |ch| {
            return Ok(PamClient::new(ch)
                .interactive_auth(InteractiveAuthRequest {
                    interactive_auth: Some(InteractiveAuth::Continue(req_inner.to_owned())),
                })
                .await?);
        }) {
            Ok(t) => t.into_inner(),
            Err(e) => {
                log::warn!("failed to continue auth: {e}");
                return PamResultCode::PAM_AUTH_ERR;
            }
        };
        iter += 1;
    }
    log::warn!("Exceeded maximum iterations");
    PamResultCode::PAM_ABORT
}
