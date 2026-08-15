//! CEF's own multi-process machinery re-execs this same binary with a
//! `--type=...` switch for renderer/GPU/utility roles; only the invocation
//! from `credprovider` (carrying `--result-pipe`/`--cancel-pipe`) becomes
//! the browser-process host that opens the sign-in window.

// Logging goes to the platform log, never stdout (see `allow_stdout(false)`
// below), so nothing needs a console. Without this the binary links as a
// console subsystem app and Windows allocates a console window for it — once
// for the host and again for every CEF renderer/GPU/utility re-exec.
#![windows_subsystem = "windows"]

mod app;
mod handler;
mod icon;
mod sysd;
mod window;

use std::path::Path;

use cef::*;

/// Chromium's on-disk state. `cache_path` is deliberately left unset so the
/// profile itself is in-memory, but this directory still accumulates state
/// across runs, and it has to be named explicitly — otherwise it lands in
/// `system32\config\systemprofile`, since the browser runs under a service
/// account at the logon screen.
const ROOT_CACHE_PATH: &str = r"C:\ProgramData\Authentik Security Inc\wcp-cache";

/// Chromium reports a failed `CHECK()` by writing the file, line and message
/// here and then executing an `int 3`, which surfaces to the credential
/// provider as nothing but exit code `0x80000003`. Its own stderr goes nowhere:
/// this is a GUI-subsystem binary with no console, and the platform log never
/// sees a message Chromium raises below the Rust layer.
const CHROMIUM_LOG_PATH: &str =
    r"C:\ProgramData\Authentik Security Inc\logs\ak_cef_chromium.log";

fn arg_value(flag: &str) -> Option<String> {
    let mut args = std::env::args();
    while let Some(a) = args.next() {
        if a == flag {
            return args.next();
        }
    }
    None
}

/// Every sign-in starts from an empty profile. The window is shown on a shared
/// logon screen, so one person's authentik session must not still be sitting
/// there for whoever is at the machine next.
///
/// Browser process only, and before `initialize`: the renderer/GPU re-execs
/// share this directory with a browser that is already using it, and CEF holds
/// files open under it once it has started.
///
/// Clears the contents rather than the directory. The installer creates it, and
/// `ProgramData` grants `CREATOR OWNER` full control over what gets created
/// beneath it — so deleting it would leave a window in which any standard user
/// on the machine could re-create it and own the directory that this process,
/// running as a service account, is about to write a browser profile into.
///
/// A failure here is logged rather than fatal: a stale cache is worth
/// reporting, but not worth refusing to let someone log in over.
fn wipe_browser_state(path: &Path) {
    if let Err(e) = std::fs::create_dir_all(path) {
        log::warn!("could not create {}: {e}", path.display());
        return;
    }

    let entries = match std::fs::read_dir(path) {
        Ok(entries) => entries,
        Err(e) => {
            log::warn!("could not read {}: {e}", path.display());
            return;
        }
    };

    let mut removed = 0usize;
    for entry in entries.flatten() {
        let entry_path = entry.path();
        let result = if entry.file_type().is_ok_and(|t| t.is_dir()) {
            std::fs::remove_dir_all(&entry_path)
        } else {
            std::fs::remove_file(&entry_path)
        };
        match result {
            Ok(()) => removed += 1,
            Err(e) => log::warn!("could not remove {}: {e}", entry_path.display()),
        }
    }
    log::info!("cleared {removed} entries of browser state in {}", path.display());
}

/// Identifies the sign-in window to authentik. Matches what the C++ credential
/// provider sent, so anything keying on it server-side keeps working.
fn user_agent() -> String {
    let cef_version = std::ffi::CStr::from_bytes_with_nul(sys::CEF_VERSION)
        .ok()
        .and_then(|v| v.to_str().ok())
        .unwrap_or("unknown");
    format!(
        "authentik Platform/WCP/CredProvider@{} (CEF {cef_version})",
        env!("CARGO_PKG_VERSION")
    )
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
    debug_assert_eq!(
        ret, -1,
        "browser process must not be handled by execute_process"
    );

    let Some(result_pipe) = arg_value("--result-pipe").and_then(|s| s.parse::<usize>().ok()) else {
        log::error!("missing --result-pipe argument");
        return;
    };
    let cancel_pipe = arg_value("--cancel-pipe").and_then(|s| s.parse::<usize>().ok());

    wipe_browser_state(Path::new(ROOT_CACHE_PATH));

    let settings = Settings {
        no_sandbox: 1,
        root_cache_path: CefString::from(ROOT_CACHE_PATH),
        user_agent: CefString::from(user_agent().as_str()),
        log_file: CefString::from(CHROMIUM_LOG_PATH),
        log_severity: LogSeverity::VERBOSE,
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

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    fn scratch_dir(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("ak_cef_{name}_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    /// A session left behind by the last person at the logon screen is the
    /// thing this is here to remove, so "mostly cleared" is not good enough:
    /// nested directories are where Chromium keeps cookies and local storage.
    #[test]
    fn clears_nested_state_but_keeps_the_directory() {
        let dir = scratch_dir("wipe");
        std::fs::create_dir_all(dir.join("Default/Local Storage")).expect("seed nested state");
        std::fs::write(dir.join("Default/Cookies"), b"session").expect("seed a cookie jar");
        std::fs::write(dir.join("Local State"), b"{}").expect("seed a loose file");

        wipe_browser_state(&dir);

        assert!(dir.is_dir(), "the directory itself must survive the wipe");
        let left: Vec<_> = std::fs::read_dir(&dir)
            .expect("read the wiped directory")
            .flatten()
            .map(|e| e.file_name())
            .collect();
        assert!(left.is_empty(), "expected an empty directory, found {left:?}");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn creates_the_directory_when_it_is_missing() {
        let dir = scratch_dir("missing");
        wipe_browser_state(&dir);
        assert!(dir.is_dir());
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// authentik may key on this server-side, so the shape is part of the
    /// contract rather than a cosmetic string.
    #[test]
    fn user_agent_carries_the_product_and_cef_versions() {
        let ua = user_agent();
        assert!(
            ua.starts_with("authentik Platform/WCP/CredProvider@"),
            "unexpected user agent: {ua}"
        );
        assert!(ua.contains(env!("CARGO_PKG_VERSION")), "{ua}");
        assert!(ua.contains("(CEF 1"), "{ua}");
        assert!(!ua.contains("unknown"), "CEF version did not decode: {ua}");
    }
}
