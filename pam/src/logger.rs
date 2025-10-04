use std::{env, ffi::CStr};

use authentik_sys::logger::log_hook;
use pam::{constants::PamFlag, module::PamHandle};

use crate::pam_env::pam_list_env;

pub fn prelude(name: &str, pamh: &mut PamHandle, args: Vec<&CStr>, _flags: PamFlag) {
    // TODO: set log level based on PAM args
    log_hook(name);
    log::debug!(
        "\tPAM args: {}",
        Vec::from_iter(args.iter().map(|i| i.to_string_lossy().into_owned())).join(", ")
    );
    log::debug!(
        "\tPAM env: {}",
        Vec::from_iter(pam_list_env(pamh).iter().map(|i| i.to_string())).join(", ")
    );
    log::debug!(
        "\tProc env: {}",
        Vec::from_iter(env::vars().map(|(k, v)| format!("{k}={v}"))).join(", ")
    );
}
