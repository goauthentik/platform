//! CEF's own multi-process machinery re-execs this same binary with a
//! `--type=...` switch for renderer/GPU/utility roles; only the invocation
//! from `credprovider` (carrying `--result-pipe`/`--cancel-pipe`) becomes
//! the browser-process host that opens the sign-in window.

mod app;
mod handler;
mod icon;
mod window;

use cef::*;

fn arg_value(flag: &str) -> Option<String> {
    let mut args = std::env::args();
    while let Some(a) = args.next() {
        if a == flag {
            return args.next();
        }
    }
    None
}

fn main() {
    ak_platform::log::LogBuilder::new(ak_platform::string::PlatformString::new_with_default(
        "ak_cef",
    ))
    .allow_platform(true)
    .allow_stdout(false)
    .enable();

    let _ = api_hash(sys::CEF_API_VERSION_LAST, 0);

    let cef_args = args::Args::new();
    let Some(cmd_line) = cef_args.as_cmd_line() else {
        log::error!("failed to parse command line");
        return;
    };
    let is_subprocess = cmd_line.has_switch(Some(&CefString::from("type"))) != 0;

    let sandbox_info = std::ptr::null_mut();
    let ret = execute_process(Some(cef_args.as_main_args()), None, sandbox_info);
    if is_subprocess {
        return;
    }
    debug_assert_eq!(ret, -1, "browser process must not be handled by execute_process");

    let Some(result_pipe) = arg_value("--result-pipe").and_then(|s| s.parse::<usize>().ok()) else {
        log::error!("missing --result-pipe argument");
        return;
    };
    let cancel_pipe = arg_value("--cancel-pipe").and_then(|s| s.parse::<usize>().ok());

    let settings = Settings {
        no_sandbox: 1,
        root_cache_path: CefString::from(r"C:\ProgramData\Authentik Security Inc\wcp-cache"),
        ..Default::default()
    };
    let mut app = app::HostApp::new(result_pipe, cancel_pipe);
    let initialized = initialize(
        Some(cef_args.as_main_args()),
        Some(&settings),
        Some(&mut app),
        sandbox_info,
    );
    if initialized != 1 {
        log::error!("CefInitialize failed");
        return;
    }

    run_message_loop();
    shutdown();
}
