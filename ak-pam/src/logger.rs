use std::{env, ffi::CStr};

use ak_platform::log::{set_log_level, LevelFilter};
use ak_platform::log::unix::log_hook;
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
    set_log_level(level);

    log_hook(name);
    tracing::debug!(
        "\tPAM args: {}",
        Vec::from_iter(args.iter().cloned()).join(", ")
    );
    tracing::debug!(
        "\tPAM env: {}",
        Vec::from_iter(pam_list_env(pamh).iter().map(|i| i.to_string())).join(", ")
    );
    tracing::debug!(
        "\tProc env: {}",
        Vec::from_iter(env::vars().map(|(k, v)| format!("{k}={v}"))).join(", ")
    );
}
