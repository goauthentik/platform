use eyre::{Result, bail};

use ak_platform::{
    generated::ic_pam_fido::{FidoRequest, FidoResponse},
    grpc::decode_pb,
};
use ctap_hid_fido2::{
    Cfg, FidoKeyHidFactory, fidokey::get_assertion::get_assertion_params::GetAssertionArgs,
};
use pam::{constants::PAM_PROMPT_ECHO_OFF, conv::Conv};

use crate::pam_print_user;

pub fn fido2(raw: String, conv: &Conv<'_>) -> Result<FidoResponse> {
    let req = decode_pb::<FidoRequest>(raw)?;

    let mut cfg = Cfg::init();
    cfg.keep_alive_msg = String::new();
    let device = FidoKeyHidFactory::create(&cfg).map_err(|e| eyre::eyre!("{e:#}"))?;

    let mut assertion_args = GetAssertionArgs {
        rpid: req.rp_id,
        challenge: req.challenge,
        pin: None,
        credential_ids: req.credential_ids,
        extensions: None,
        // User verification
        uv: None,
        // User presence
        up: true,
    };

    let pin_cstring: Option<String> = if req.uv {
        match conv.send(PAM_PROMPT_ECHO_OFF, "Input Security key PIN: ") {
            Ok(c) => match c {
                Some(c) => match c.as_str() {
                    Ok(d) => Some(d.to_string()),
                    Err(e) => {
                        log::warn!("failed to convert pin to string: {e:?}");
                        return Err(e.into());
                    }
                },
                None => {
                    log::warn!("Failed to get PIN");
                    bail!("failed to get pin");
                }
            },
            Err(_) => bail!("failed to get pin"),
        }
    } else {
        None
    };

    if let Some(ref pc) = pin_cstring {
        assertion_args.pin = Some(pc);
        assertion_args.uv = None;
    }

    pam_print_user(conv, "Touch your security key...");
    let _ = device.wink();

    let assertions = device
        .get_assertion_with_args(&assertion_args)
        .map_err(|e| eyre::eyre!("{e:#}"))?;
    log::debug!("FIDO2: Authenticate Success");

    pam_print_user(conv, "Validating...");

    Ok(FidoResponse {
        credential_id: assertions[0].credential_id.clone(),
        signature: assertions[0].signature.clone(),
        authenticator_data: assertions[0].auth_data.clone(),
    })
}
