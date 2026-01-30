use std::error::Error;

use authentik_sys::{
    generated::ssh::{FidoRequest, FidoResponse},
    grpc::decode_pb,
};
use ctap_hid_fido2::{Cfg, FidoKeyHidFactory, fidokey::GetAssertionArgsBuilder};
use pam::{
    constants::{PAM_PROMPT_ECHO_OFF, PAM_TEXT_INFO},
    conv::Conv,
};

pub fn fido2(raw: String, conv: &Conv<'_>) -> Result<FidoResponse, Box<dyn Error>> {
    let req = decode_pb::<FidoRequest>(raw)?;
    let pin = match conv.send(PAM_PROMPT_ECHO_OFF, "Input Security key PIN: ") {
        Ok(c) => match c {
            Some(c) => match c.to_str() {
                Ok(cc) => cc,
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

    let mut cfg = Cfg::init();
    cfg.keep_alive_msg = String::new();
    let device = FidoKeyHidFactory::create(&cfg)?;

    let mut get_assertion_args = GetAssertionArgsBuilder::new(&req.rp_id, &req.challenge).pin(pin);

    for cred_id in req.credential_ids.iter() {
        get_assertion_args = get_assertion_args.add_credential_id(cred_id);
    }
    let assertion_args = get_assertion_args.build();

    let _ = conv.send(PAM_TEXT_INFO, "Touch your security key");

    let assertions = device.get_assertion_with_args(&assertion_args)?;
    log::debug!("- Authenticate Success");

    Ok(FidoResponse {
        credential_id: assertions[0].credential_id.clone(),
        signature: assertions[0].signature.clone(),
        authenticator_data: assertions[0].auth_data.clone(),
    })
}
