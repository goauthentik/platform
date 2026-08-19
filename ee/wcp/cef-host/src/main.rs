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
/// Never passed to CEF directly — see `browser_state_dir`. Sharing one fixed
/// `root_cache_path` across launches would let an overlapping or leftover
/// instance block every subsequent one via `ProcessSingleton`.
const CACHE_ROOT: &str = r"C:\ProgramData\Authentik Security Inc\wcp-cache";

/// Chromium reports a failed `CHECK()` by writing the file, line and message
/// here and then executing an `int 3`, which surfaces to the credential
/// provider as nothing but exit code `0x80000003`. Its own stderr goes nowhere:
/// this is a GUI-subsystem binary with no console, and the platform log never
/// sees a message Chromium raises below the Rust layer.
const CHROMIUM_LOG_PATH: &str = r"C:\ProgramData\Authentik Security Inc\logs\ak_cef_chromium.log";

/// `::CreateMutex(NULL, FALSE, name)`, logging the result through a channel
/// already confirmed to work — Chromium's own diagnostic for this exact call
/// (inside `ProcessSingleton::Create()`) is `DPLOG(FATAL)`, which compiles to
/// nothing in a release build, so it stays silent regardless of log
/// verbosity.
fn try_create_mutex(label: &str, name: &str) {
    use windows::Win32::Foundation::CloseHandle;
    use windows::Win32::System::Threading::CreateMutexW;
    use windows::core::PCWSTR;

    let name_wide: Vec<u16> = name.encode_utf16().chain(std::iter::once(0)).collect();
    match unsafe { CreateMutexW(None, false, PCWSTR(name_wide.as_ptr())) } {
        Ok(handle) => {
            log::info!("diagnostic: CreateMutex({label}) succeeded");
            unsafe {
                let _ = CloseHandle(handle);
            }
        }
        Err(e) => log::error!("diagnostic: CreateMutex({label}) failed: {e}"),
    }
}

/// The exact call CEF's `ProcessSingleton::Create()` makes, confirmed
/// failing with `ERROR_ACCESS_DENIED` against a real install, on a fresh
/// create with nothing pre-existing (`handle.exe` found no stale object).
/// The three follow-up calls narrow down why: a random name under the same
/// `Local\` session namespace tells apart "this token cannot create named
/// mutexes in the session namespace at all" from "this specific,
/// publicly-documented name is blocked" (endpoint security software is the
/// leading suspect for the latter — it is exactly the kind of string a
/// malware/injection heuristic keys on); a `Global\` name checks whether the
/// restriction is specific to the per-session namespace; a name containing
/// "Chrome" but not the exact reserved string checks whether the block
/// matches on the literal known string or more loosely on the product name.
fn diagnose_process_singleton_mutex() {
    try_create_mutex(
        "ChromeProcessSingletonStartup!",
        "Local\\ChromeProcessSingletonStartup!",
    );
    try_create_mutex(
        "random, Local",
        &format!("Local\\ak-cef-diagnostic-{}", uuid::Uuid::new_v4()),
    );
    try_create_mutex(
        "random, Global",
        &format!("Global\\ak-cef-diagnostic-{}", uuid::Uuid::new_v4()),
    );
    try_create_mutex(
        "Chrome-ish but not the reserved string",
        &format!("Local\\ChromeDiagnosticProbe-{}", uuid::Uuid::new_v4()),
    );
}

/// Logs this process's own token identity, mandatory integrity level,
/// session id, and whether Windows considers it a *restricted* token
/// (`IsTokenRestricted`, true only if `CreateRestrictedToken` was given SIDs
/// to disable or restrict — `service_account_token` gives it neither, only
/// `DISABLE_MAX_PRIVILEGE`, so this is expected to read false; confirming
/// that rules out the double access-check restricted tokens get as an
/// explanation for the `CreateMutex` failure above).
fn diagnose_token() {
    use std::ffi::c_void;
    use windows::Win32::Foundation::{CloseHandle, HANDLE};
    use windows::Win32::Security::{
        GetSidSubAuthority, GetSidSubAuthorityCount, GetTokenInformation, IsTokenRestricted,
        TOKEN_MANDATORY_LABEL, TOKEN_QUERY, TokenIntegrityLevel, TokenSessionId,
    };
    use windows::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

    let mut token = HANDLE::default();
    if unsafe { OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) }.is_err() {
        log::error!("diagnostic: could not open this process's own token");
        return;
    }

    let mut label_buf = [0u8; 64];
    let mut ret_len = 0u32;
    let got_label = unsafe {
        GetTokenInformation(
            token,
            TokenIntegrityLevel,
            Some(label_buf.as_mut_ptr() as *mut c_void),
            label_buf.len() as u32,
            &mut ret_len,
        )
    };
    if got_label.is_ok() {
        let label = unsafe { &*(label_buf.as_ptr() as *const TOKEN_MANDATORY_LABEL) };
        let sid = label.Label.Sid;
        let rid = unsafe {
            let count = *GetSidSubAuthorityCount(sid);
            *GetSidSubAuthority(sid, (count - 1) as u32)
        };
        let level = match rid {
            0x0000 => "Untrusted",
            0x1000 => "Low",
            0x2000 => "Medium",
            0x2100 => "Medium Plus",
            0x3000 => "High",
            0x4000 => "System",
            _ => "Unknown",
        };
        log::info!("diagnostic: token integrity level = {level} (rid {rid:#06x})");
    } else {
        log::error!("diagnostic: could not read token integrity level");
    }

    let mut session_id: u32 = u32::MAX;
    let mut ret_len2 = 0u32;
    let got_session = unsafe {
        GetTokenInformation(
            token,
            TokenSessionId,
            Some(std::ptr::from_mut(&mut session_id).cast()),
            size_of::<u32>() as u32,
            &mut ret_len2,
        )
    };
    if got_session.is_ok() {
        log::info!("diagnostic: token session id = {session_id}");
    } else {
        log::error!("diagnostic: could not read token session id");
    }

    // `IsTokenRestricted` reports via Result rather than a bool: Ok means the
    // token carries a restricted-SIDs list (the raw BOOL was TRUE), Err means
    // it does not.
    let restricted = unsafe { IsTokenRestricted(token) }.is_ok();
    log::info!("diagnostic: IsTokenRestricted = {restricted}");

    unsafe {
        let _ = CloseHandle(token);
    }
}

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

/// Every sign-in starts from an empty profile, and none should linger on
/// disk once its window closes — the logon screen is shared. Only called
/// after a successful run: this run's directory is unique to it
/// (`browser_state_dir`), so there is nothing to clear before starting, and
/// a directory `CefInitialize` never got to use is left in place
/// deliberately, as the only record of what that run looked like when it
/// failed. Also skipped if the credential provider has to kill this process
/// for never responding — logged, not fatal, like any other cleanup here.
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

    diagnose_token();
    diagnose_process_singleton_mutex();

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
