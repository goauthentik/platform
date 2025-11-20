use authentik_sys::generated::{
    grpc_request,
    sys_auth::{
        InteractiveAuthContinueRequest, InteractiveAuthInitRequest, InteractiveAuthRequest,
        InteractiveAuthResult, InteractiveChallenge, interactive_auth_request::InteractiveAuth,
        interactive_challenge::PromptMeta,
        system_auth_interactive_client::SystemAuthInteractiveClient,
    },
};
use pam::{
    constants::{
        PAM_BINARY_PROMPT, PAM_ERROR_MSG, PAM_PROMPT_ECHO_OFF, PAM_PROMPT_ECHO_ON, PAM_RADIO_TYPE,
        PAM_TEXT_INFO, PamMessageStyle, PamResultCode,
    },
    conv::Conv,
};

use crate::auth::PW_PROMPT;

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
) -> Result<InteractiveChallenge, PamResultCode> {
    // Init transaction
    let mut challenge = match grpc_request(async |ch| {
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
    };
    let mut iter = 0;
    while iter <= MAX_ITER {
        log::debug!("{} processing challenge: {:?}", iter, challenge);
        if challenge.finished {
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
        challenge = match grpc_request(async |ch| {
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
        iter += 1;
    }
    log::warn!("Exceeded maximum iterations");
    Err(PamResultCode::PAM_ABORT)
}
