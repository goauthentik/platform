use std::error::Error;

use authentik_sys::{
    generated::ssh::{FidoRequest, FidoResponse},
    grpc::decode_pb,
};
use ctap_hid_fido2::{Cfg, FidoKeyHidFactory, fidokey::GetAssertionArgsBuilder};

pub fn fido2(raw: String) -> Result<FidoResponse, Box<dyn Error>> {
    let req = decode_pb::<FidoRequest>(raw)?;
    let pin = get_input_with_message("input PIN:");

    let device = FidoKeyHidFactory::create(&Cfg::init())?;

    let mut get_assertion_args = GetAssertionArgsBuilder::new(&req.rp_id, &req.challenge).pin(&pin);

    for cred_id in req.credential_ids.iter() {
        get_assertion_args = get_assertion_args.add_credential_id(cred_id);
    }
    let assertion_args = get_assertion_args.build();

    let assertions = device.get_assertion_with_args(&assertion_args)?;
    log::debug!("- Authenticate Success");

    Ok(FidoResponse {
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
