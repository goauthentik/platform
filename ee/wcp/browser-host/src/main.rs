//! The sign-in window the credential provider opens on the Windows logon
//! screen, hosted on WebView2 through Tauri.
//!
//! Spawned by `credprovider` with the sign-in URL and header token it already
//! resolved (`--sign-in-url`/`--header-token`), and with its inherited
//! stdin/stdout as the IPC channel. Unlike the CEF host this replaces there is
//! no re-exec of this binary for renderer/GPU roles — WebView2 runs its own
//! `msedgewebview2.exe` children — so every invocation is the browser host.

// Logging goes to the platform log, never stdout: stdout *is* the result pipe
// here, so anything written to it would corrupt the frame the credential
// provider is parsing. Without the subsystem attribute the binary links as a
// console app and Windows allocates a console window for it.
#![windows_subsystem = "windows"]

mod foreground;
mod icon;
mod identity;
mod signin;
mod webview2;

use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use ak_ee_wcp_wire::HostReport;
use tauri::{WebviewUrl, WebviewWindowBuilder, WindowEvent};

/// Parent of WebView2's on-disk state. Named explicitly because the browser
/// runs under a service account at the logon screen: left to itself WebView2
/// puts its user-data folder next to the executable in `Program Files`, which
/// that account cannot write to. The installer grants the account full control
/// of this directory (`vpkg/windows/Package.wxs`), so keep the two in step.
///
/// Never used directly as the user-data folder — see `browser_state_dir`.
const CACHE_ROOT: &str = r"C:\ProgramData\Authentik Security Inc\wcp-cache";

fn arg_value(flag: &str) -> Option<String> {
    let mut args = std::env::args();
    while let Some(a) = args.next() {
        if a == flag {
            return args.next();
        }
    }
    None
}

/// A fresh, unique directory under `root` for exactly this run's user-data
/// folder. Owned outright by this process — nothing pre-creates it the way the
/// installer pre-creates `root` — so `wipe_browser_state` can remove it
/// entirely afterwards rather than only clearing its contents.
///
/// A single fixed folder shared across launches is what let one leftover
/// process lock out every subsequent one under CEF, and WebView2 is no
/// friendlier: a user-data folder is single-writer, and an environment pointed
/// at one another process still holds fails to create at all.
fn browser_state_dir(root: &Path) -> std::path::PathBuf {
    let path = root.join(uuid::Uuid::new_v4().to_string());
    match std::fs::create_dir_all(&path) {
        Ok(()) => log::info!("using {} as this run's user-data folder", path.display()),
        Err(e) => log::warn!("could not create {}: {e}", path.display()),
    }
    path
}

/// Every sign-in starts from an empty profile, and none should linger on disk
/// once its window closes — the logon screen is shared. This run's directory
/// is unique to it (`browser_state_dir`), so there is nothing to clear
/// beforehand. Logged, not fatal.
fn wipe_browser_state(path: &Path) {
    if let Err(e) = std::fs::remove_dir_all(path) {
        log::warn!("could not remove {}: {e}", path.display());
    }
}

/// Identifies the sign-in window to authentik. Matches what the C++ credential
/// provider sent, so anything keying on it server-side keeps working, with the
/// version coming from `ak-meta` rather than the crate — it is the build's own
/// version, and it carries the build hash the rest of the platform reports.
fn user_agent(runtime_version: &str) -> String {
    format!(
        "authentik Platform/WCP/CredProvider@{} (WebView2 {runtime_version})",
        ak_meta::full_version()
    )
}

/// Scheme and host of `url`, or a placeholder — the rest of a sign-in URL is
/// not safe to log.
fn origin_of(url: &str) -> String {
    url.parse::<tauri::Url>()
        .ok()
        .and_then(|parsed| {
            parsed
                .host_str()
                .map(|host| format!("{}://{host}", parsed.scheme()))
        })
        .unwrap_or_else(|| "<unparseable URL>".to_string())
}

/// Opens the window and runs the message loop until the flow ends.
///
/// Returns the report to fall back on rather than sending it: `main` sends
/// whatever comes back, and `Completion::send` is send-once, so a route that
/// already answered — a redirect, a cancellation — wins over this.
fn run(
    completion: &Arc<signin::Completion>,
    cancel_pipe: Option<std::fs::File>,
    sign_in_url: String,
    header_token: String,
    cache_path: &Path,
) -> HostReport {
    let Some(runtime_version) = webview2::runtime_version() else {
        log::error!(
            "the Microsoft Edge WebView2 runtime is not installed; there is nothing to render \
             the sign-in window with"
        );
        return HostReport::Cancelled;
    };
    log::info!("WebView2 runtime {runtime_version} found");

    let url = match sign_in_url.parse::<tauri::Url>() {
        Ok(url) => url,
        Err(e) => {
            log::error!("the sign-in URL from credprovider does not parse: {e}");
            return HostReport::Cancelled;
        }
    };
    log::info!("sign-in URL: {}", origin_of(&sign_in_url));

    // Whether the window has ever held the foreground, which separates "never
    // took focus" from "took it and lost it again". Fed by the `Focused`
    // events below and read by the retry ladder when it gives up.
    let ever_activated = Arc::new(AtomicBool::new(false));

    let setup_completion = completion.clone();
    let setup_activated = ever_activated.clone();
    let event_completion = completion.clone();
    let data_directory = cache_path.to_path_buf();

    let ran = tauri::Builder::default()
        .setup(move |app| {
            let handle = app.handle().clone();
            let nav_completion = setup_completion.clone();
            let nav_handle = handle.clone();

            let mut builder =
                WebviewWindowBuilder::new(app, "sign-in", WebviewUrl::External(url.clone()))
                    .title("Sign in with authentik")
                    .inner_size(
                        f64::from(ak_ee_wcp_wire::WINDOW_WIDTH),
                        f64::from(ak_ee_wcp_wire::WINDOW_HEIGHT),
                    )
                    .center()
                    .resizable(false)
                    .minimizable(false)
                    .maximizable(false)
                    // Topmost before the first paint, so the window is never
                    // behind LogonUI even for a frame. Asking for focus is a
                    // separate and much less certain thing — see `foreground`.
                    .always_on_top(true)
                    .focused(true)
                    .user_agent(&user_agent(&runtime_version))
                    .data_directory(data_directory.clone())
                    .incognito(true)
                    .on_navigation(move |url| {
                        let url = url.as_str();
                        if !url.starts_with(ak_ee_wcp_wire::REDIRECT_PREFIX) {
                            log::debug!("navigating to {}", origin_of(url));
                            return true;
                        }

                        // Validating the token in this URL needs `ak-sysd`,
                        // which this process has no access to; `credprovider`
                        // does it once this reaches the result pipe.
                        log::info!("redirect detected; reporting it for validation");
                        nav_completion.send(HostReport::Redirected {
                            url: url.to_string(),
                        });
                        signin::close(&nav_handle);
                        // WebView2 cannot navigate to `goauthentik.io://`
                        // anyway; cancelling keeps it from saying so in the
                        // window the person is still looking at.
                        false
                    });

            match icon::load() {
                Some(image) => builder = builder.icon(image)?,
                None => log::warn!("the window icon did not decode; opening without one"),
            }

            let window = builder.build()?;
            log::info!(
                "sign-in window built, foreground_pid={:?}",
                foreground::foreground_pid()
            );

            webview2::inject_header(&window, header_token.clone());

            match window.hwnd() {
                Ok(hwnd) => foreground::watch(hwnd.0 as isize, setup_activated.clone()),
                Err(e) => log::error!("no window handle to keep in the foreground: {e}"),
            }

            if let Some(cancel_pipe) = cancel_pipe {
                signin::watch_cancel_pipe(cancel_pipe, handle, setup_completion.clone());
            }

            Ok(())
        })
        .on_window_event(move |_window, event| match event {
            WindowEvent::Focused(focused) => {
                if *focused {
                    ever_activated.store(true, Ordering::SeqCst);
                }
                log::info!(
                    "sign-in window focus changed: focused={focused} foreground_pid={:?}",
                    foreground::foreground_pid()
                );
            }
            WindowEvent::Destroyed => {
                log::info!("sign-in window destroyed");
                // A no-op on every route that already reported; the one it
                // exists for is the person closing the window.
                event_completion.send(HostReport::Cancelled);
            }
            _ => {}
        })
        .run(tauri::generate_context!());

    if let Err(e) = ran {
        log::error!("the sign-in window could not run: {e}");
    }
    HostReport::Cancelled
}

fn main() {
    // Held for the whole of `main` so the client is still alive to flush when
    // the process exits.
    let _sentry = sentry::init(ak_meta::sentry_options("ak-browser"));

    ak_platform::log::LogBuilder::new(ak_platform::string::PlatformString::new_with_default(
        "authentik Credential Provider (Browser)",
    ))
    .with_default_filters()
    .allow_platform(true)
    .allow_stdout(false)
    .enable();

    // First line out, before anything else can fail: which exact commit this
    // binary was built from and which account it is running as, so a real
    // install's log can be matched against the source rather than assumed.
    log::info!(
        "ak_browser.exe {} (build {}), running as {}",
        ak_meta::full_version(),
        ak_meta::build_hash(),
        identity::current_token_identity()
    );

    let Some(pipes) = signin::inherited_pipes() else {
        return;
    };
    let completion = Arc::new(signin::Completion::new(pipes.result));

    let Some(sign_in_url) = arg_value("--sign-in-url") else {
        log::error!("missing --sign-in-url argument");
        completion.send(HostReport::Cancelled);
        return;
    };
    let Some(header_token) = arg_value("--header-token") else {
        log::error!("missing --header-token argument");
        completion.send(HostReport::Cancelled);
        return;
    };

    let cache_path = browser_state_dir(Path::new(CACHE_ROOT));

    let report = run(
        &completion,
        pipes.cancel,
        sign_in_url,
        header_token,
        &cache_path,
    );
    // Whatever happened, the credential provider gets exactly one answer.
    completion.send(report);
    wipe_browser_state(&cache_path);
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    fn scratch_dir(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("ak_browser_{name}_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    /// A session left behind by the last person at the logon screen is the
    /// thing this is here to remove, so a real recursive remove is required —
    /// nested directories are where a browser keeps cookies and local storage.
    /// The directory itself goes too: each run owns its own
    /// (`browser_state_dir`), so nothing else still needs it afterwards.
    #[test]
    fn removes_a_populated_directory_entirely() {
        let dir = scratch_dir("wipe");
        std::fs::create_dir_all(dir.join("Default/Local Storage")).expect("seed nested state");
        std::fs::write(dir.join("Default/Cookies"), b"session").expect("seed a cookie jar");
        std::fs::write(dir.join("Local State"), b"{}").expect("seed a loose file");

        wipe_browser_state(&dir);

        assert!(!dir.exists(), "the directory itself should be gone");
    }

    /// Reached whenever the host exits before ever creating its own directory
    /// (e.g. a missing `--sign-in-url`) — must not panic.
    #[test]
    fn tolerates_a_directory_that_is_already_gone() {
        let dir = scratch_dir("missing");
        wipe_browser_state(&dir);
        assert!(!dir.exists());
    }

    /// A WebView2 user-data folder is single-writer: pointing an environment
    /// at one another process still holds fails to create at all, which is the
    /// whole reason each run gets its own.
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
    fn user_agent_carries_the_product_and_runtime_versions() {
        let ua = user_agent("120.0.2210.91");
        let rest = ua
            .strip_prefix("authentik Platform/WCP/CredProvider@")
            .expect("the C++ user-agent prefix");
        let (version, runtime) = rest.split_once(" (WebView2 ").expect("a WebView2 section");

        assert_eq!(version, ak_meta::full_version(), "not ak-meta's version");
        assert_eq!(runtime, "120.0.2210.91)");
    }

    /// Sign-in URLs carry tokens in the path and query, so only the origin is
    /// ever safe to put in a log line.
    #[test]
    fn only_the_origin_is_logged() {
        assert_eq!(
            origin_of("https://authentik.company/if/flow/default/?token=secret"),
            "https://authentik.company"
        );
        assert_eq!(origin_of("not a url"), "<unparseable URL>");
    }
}
