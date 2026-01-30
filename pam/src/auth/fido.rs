use std::error::Error;

use authentik_sys::{
    generated::ssh::{FidoRequest, FidoResponse},
    grpc::decode_pb,
};
use ctap_hid_fido2::{Cfg, FidoKeyHidFactory, fidokey::GetAssertionArgsBuilder};
use pam::{constants::PAM_TEXT_INFO, conv::Conv};

pub fn fido2(raw: String, conv: &Conv<'_>) -> Result<FidoResponse, Box<dyn Error>> {
    let req = decode_pb::<FidoRequest>(raw)?;
    let pin = get_input_with_message("input PIN:");

    let devs = ctap_hid_fido2::get_fidokey_devices();
    for dev in devs {
        log::debug!(
            "- vid=0x{:04x} , pid=0x{:04x} , info={:?}",
            dev.vid,
            dev.pid,
            dev.info
        );

        let fidokey = FidoKeyHidFactory::create_by_params(&vec![dev.param], &Cfg::init()).unwrap();
        let info = fidokey.get_info().unwrap();
        log::debug!("{}", info);
    }

    let mut cfg = Cfg::init();
    cfg.keep_alive_msg = String::new();
    let device = FidoKeyHidFactory::create(&cfg)?;

    let mut get_assertion_args =
        GetAssertionArgsBuilder::new(&req.rp_id, &req.challenge)
        .pin(&pin);

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

pub fn get_input() -> String {
    let mut word = String::new();
    std::io::stdin().read_line(&mut word).ok();
    word.trim().to_string()
}

pub fn get_input_with_message(message: &str) -> String {
    println!("{}", message);
    let input = get_input();
    println!();
    input
}
