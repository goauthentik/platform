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
mod foreground;
mod handler;
mod icon;
mod sysd;
mod window;

use std::path::Path;

use cef::*;

/// Parent of Chromium's on-disk state. `cache_path` is deliberately left
/// unset so the profile itself is in-memory, but `root_cache_path` still
/// accumulates state across runs, and it has to be named explicitly —
/// otherwise it lands in `system32\config\systemprofile`, since the browser
/// runs under a service account at the logon screen.
///
/// Never passed to CEF directly — see `browser_state_dir`. `root_cache_path`
/// doubles as Chromium's user-data directory, which is what
/// `ProcessSingleton` derives its cross-process lock from; sharing one fixed
/// path across launches means a second sign-in attempt starting before the
/// first one's process has fully torn down — or any leftover instance from
/// an earlier run — fails `CefInitialize` outright rather than opening a
/// second window (`ProcessSingleton` exists specifically to prevent that).
const CACHE_ROOT: &str = r"C:\ProgramData\Authentik Security Inc\wcp-cache";

/// Chromium reports a failed `CHECK()` by writing the file, line and message
/// here and then executing an `int 3`, which surfaces to the credential
/// provider as nothing but exit code `0x80000003`. Its own stderr goes nowhere:
/// this is a GUI-subsystem binary with no console, and the platform log never
/// sees a message Chromium raises below the Rust layer.
const CHROMIUM_LOG_PATH: &str = r"C:\ProgramData\Authentik Security Inc\logs\ak_cef_chromium.log";

fn arg_value(flag: &str) -> Option<String> {
    let mut args = std::env::args();
    while let Some(a) = args.next() {
        if a == flag {
            return args.next();
        }
    }
    None
}

/// A fresh, unique directory under `root` for exactly this run's
/// `root_cache_path` — see `CACHE_ROOT`'s doc for why it cannot be `root`
/// itself. Owned outright by this process (nothing pre-creates it the way
/// the installer pre-creates `root`), so `wipe_browser_state` can remove it
/// entirely once this run is done, rather than only clearing its contents.
fn browser_state_dir(root: &Path) -> std::path::PathBuf {
    let path = root.join(uuid::Uuid::new_v4().to_string());
    match std::fs::create_dir_all(&path) {
        Ok(()) => log::info!("using {} as this run's root_cache_path", path.display()),
        Err(e) => log::warn!("could not create {}: {e}", path.display()),
    }
    path
}

/// Every sign-in starts from an empty profile, and none should linger on disk
/// once its window has closed — the logon screen is shared, so one person's
/// session must not leak into the next.
///
/// Only called after a *successful* run, never at the top of `main` and
/// never after `CefInitialize` itself fails: this run's directory is unique
/// to it (`browser_state_dir`), so there is nothing to clear before
/// starting, and a directory `CefInitialize` never got to use is left in
/// place deliberately, as the only record of what that run's environment
/// actually looked like when it failed. If `ak_cef.exe` is killed rather
/// than exiting normally — the credential provider gives up and terminates
/// it when the sign-in window never responds — this never runs either, and
/// the directory is simply left behind; logged, not fatal, the same as any
/// other cleanup failure here.
fn wipe_browser_state(path: &Path) {
    if let Err(e) = std::fs::remove_dir_all(path) {
        log::warn!("could not remove {}: {e}", path.display());
    }
}

/// Identifies the sign-in window to authentik. Matches what the C++ credential
/// provider sent, so anything keying on it server-side keeps working, with the
/// version coming from `ak-meta` rather than the crate — it is the build's own
/// version, and it carries the build hash the rest of the platform reports.
fn user_agent() -> String {
    let cef_version = std::ffi::CStr::from_bytes_with_nul(sys::CEF_VERSION)
        .ok()
        .and_then(|v| v.to_str().ok())
        .unwrap_or("unknown");
    format!(
        "authentik Platform/WCP/CredProvider@{} (CEF {cef_version})",
        ak_meta::full_version()
    )
}

fn main() {
    // Held for the whole of `main` so the client is still alive to flush when
    // the process exits. Initialised before the `--type` branch below, so the
    // renderer/GPU/utility re-execs report too — a crash in one of those is
    // otherwise invisible, which is exactly the position this flow was in.
    let _sentry = sentry::init(ak_meta::sentry_options("ak-cef"));

    ak_platform::log::LogBuilder::new(ak_platform::string::PlatformString::new_with_default(
        "authentik Credential Provider (CEF)",
    ))
    .with_default_filters()
    .allow_platform(true)
    .allow_stdout(false)
    .enable();

    // First line out, before anything else can fail: which exact commit this
    // binary was built from, so a real-install log can be matched against
    // the source rather than assumed.
    log::info!(
        "ak_cef.exe {} (build {})",
        ak_meta::full_version(),
        ak_meta::build_hash()
    );

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

    let Some(result_pipe) = arg_value("--result-pipe") else {
        log::error!("missing --result-pipe argument");
        return;
    };
    let cancel_pipe = arg_value("--cancel-pipe");

    let cache_path = browser_state_dir(Path::new(CACHE_ROOT));

    let settings = Settings {
        no_sandbox: 1,
        root_cache_path: CefString::from(cache_path.to_string_lossy().as_ref()),
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
        log::error!(
            "CefInitialize failed; leaving {} in place for inspection",
            cache_path.display()
        );
        return;
    }

    run_message_loop();
    shutdown();
    wipe_browser_state(&cache_path);
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
    /// thing this is here to remove, so a real recursive remove is required —
    /// nested directories are where Chromium keeps cookies and local storage.
    /// Unlike the old shared-directory design, the directory itself goes too:
    /// each run owns its own (`browser_state_dir`), so there is nothing else
    /// that still needs it to exist afterwards.
    #[test]
    fn removes_a_populated_directory_entirely() {
        let dir = scratch_dir("wipe");
        std::fs::create_dir_all(dir.join("Default/Local Storage")).expect("seed nested state");
        std::fs::write(dir.join("Default/Cookies"), b"session").expect("seed a cookie jar");
        std::fs::write(dir.join("Local State"), b"{}").expect("seed a loose file");

        wipe_browser_state(&dir);

        assert!(!dir.exists(), "the directory itself should be gone");
    }

    /// Reached whenever `ak_cef.exe` exits before ever creating its own
    /// directory (e.g. a missing `--result-pipe` argument) — must not panic.
    #[test]
    fn tolerates_a_directory_that_is_already_gone() {
        let dir = scratch_dir("missing");
        wipe_browser_state(&dir);
        assert!(!dir.exists());
    }

    /// A shared, unchanging `root_cache_path` is what let one leftover
    /// process's `ProcessSingleton` lock fail every subsequent launch — the
    /// whole reason each run gets its own directory instead of reusing
    /// `root` directly.
    #[test]
    fn each_call_gets_its_own_directory() {
        let root = scratch_dir("state_dir");

        let first = browser_state_dir(&root);
        let second = browser_state_dir(&root);

        assert_ne!(first, second, "each launch must get a distinct directory");
        assert!(first.is_dir());
        assert!(second.is_dir());

        let _ = std::fs::remove_dir_all(&root);
    }

    /// authentik may key on this server-side, so the shape is part of the
    /// contract rather than a cosmetic string. The product version is whatever
    /// `ak-meta` was built with — `make ee/wcp/test` does not set `AK_VERSION`
    /// the way `make ee/wcp/build` does, so assert it is reported faithfully
    /// rather than asserting it is non-empty.
    #[test]
    fn user_agent_carries_the_product_and_cef_versions() {
        let ua = user_agent();
        let rest = ua
            .strip_prefix("authentik Platform/WCP/CredProvider@")
            .expect("the C++ user-agent prefix");
        let (version, cef) = rest.split_once(" (CEF ").expect("a CEF section");

        assert_eq!(version, ak_meta::full_version(), "not ak-meta's version");
        let cef = cef.strip_suffix(')').expect("a closing parenthesis");
        assert!(
            cef.starts_with(|c: char| c.is_ascii_digit()),
            "CEF version did not decode: {ua}"
        );
    }
}
