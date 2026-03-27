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
