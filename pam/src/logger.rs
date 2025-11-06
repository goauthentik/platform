use std::{env, ffi::CStr};

use authentik_sys::logger::{init_log_level, log_hook};
use log::LevelFilter;
use pam::{constants::PamFlag, module::PamHandle};

use crate::pam_env::pam_list_env;

pub fn prelude(name: &str, pamh: &mut PamHandle, args: Vec<&CStr>, _flags: PamFlag) {
    let args: Vec<_> = args
        .iter()
        .map(|s| s.to_string_lossy().into_owned())
        .collect();
    let mut level = LevelFilter::Info;
    if args.contains(&"debug".to_string()) {
        level = LevelFilter::Debug;
    }
    init_log_level("libpam-authentik", level);

    log_hook(name);
    log::debug!(
        "\tPAM args: {}",
        Vec::from_iter(args.iter().cloned()).join(", ")
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
