use std::error::Error;

use authentik_sys::{
    generated::ic_pam_fido::{FidoRequest, FidoResponse},
    grpc::decode_pb,
};
use ctap_hid_fido2::{
    Cfg, FidoKeyHidFactory, fidokey::get_assertion::get_assertion_params::GetAssertionArgs,
};
use pam::{constants::PAM_PROMPT_ECHO_OFF, conv::Conv};

use crate::pam_print_user;

pub fn fido2(raw: String, conv: &Conv<'_>) -> Result<FidoResponse, Box<dyn Error>> {
    let req = decode_pb::<FidoRequest>(raw)?;

    let mut cfg = Cfg::init();
    cfg.keep_alive_msg = String::new();
    let device = FidoKeyHidFactory::create(&cfg)?;

    let mut assertion_args = GetAssertionArgs {
        rpid: req.rp_id,
        challenge: req.challenge,
        pin: None,
        uv: None,
        credential_ids: req.credential_ids,
        extensions: None,
    };

    if req.uv {
        match conv.send(PAM_PROMPT_ECHO_OFF, "Input Security key PIN: ") {
            Ok(c) => match c {
                Some(c) => match c.to_str() {
                    Ok(cc) => {
                        assertion_args.pin = Some(cc);
                        assertion_args.uv = None;
                    }
                    Err(e) => return Err(Box::from(e)),
                },
                None => {
                    log::warn!("Failed to get PIN");
                    return Err(Box::from("failed to get pin"));
                }
            },
            Err(_) => {
                return Err(Box::from("failed to get pin"));
            }
        };
    }

    pam_print_user(conv, "Touch your security key...");
    let _ = device.wink();

    let assertions = device.get_assertion_with_args(&assertion_args)?;
    log::debug!("FIDO2: Authenticate Success");

    pam_print_user(conv, "Validating...");

    Ok(FidoResponse {
        credential_id: assertions[0].credential_id.clone(),
        signature: assertions[0].signature.clone(),
        authenticator_data: assertions[0].auth_data.clone(),
    })
}
