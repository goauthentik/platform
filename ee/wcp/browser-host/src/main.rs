//! The sign-in window the credential provider opens on the Windows logon
//! screen, hosted on WebView2 through Tauri. Spawned by `credprovider` with
//! its inherited stdin/stdout as the IPC channel.

// Otherwise the binary links as a console app and Windows allocates a console
// window. Nothing may write to stdout either: it is the result pipe.
#![windows_subsystem = "windows"]

mod foreground;
mod identity;
mod signin;
mod webview2;

use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use ak_ee_wcp_wire::HostReport;
use tauri::webview::PageLoadEvent;
use tauri::{Manager, WebviewUrl, WebviewWindowBuilder, WindowEvent};

/// Parent of WebView2's on-disk state. Named explicitly because the service
/// account this runs as cannot write next to the executable, where WebView2
/// would otherwise put its user-data folder. The installer grants that account
/// full control of this directory (`vpkg/windows/Package.wxs`), so keep the two
/// in step. The folder itself is a child of this — see `browser_state_dir`.
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

/// A fresh directory per run, owned outright by this process so
/// `wipe_browser_state` can remove it entirely afterwards. A WebView2
/// user-data folder is single-writer, so one folder shared across launches
/// lets a leftover process lock out every subsequent one.
fn browser_state_dir(root: &Path) -> std::path::PathBuf {
    let path = root.join(uuid::Uuid::new_v4().to_string());
    match std::fs::create_dir_all(&path) {
        Ok(()) => log::info!("using {} as this run's user-data folder", path.display()),
        Err(e) => log::warn!("could not create {}: {e}", path.display()),
    }
    path
}

/// The logon screen is shared, so no profile may linger on disk once its
/// window closes. Logged, not fatal.
fn wipe_browser_state(path: &Path) {
    if let Err(e) = std::fs::remove_dir_all(path) {
        log::warn!("could not remove {}: {e}", path.display());
    }
}

/// Identifies the sign-in window to authentik. Matches what the C++ credential
/// provider sent, so anything keying on it server-side keeps working. The
/// version comes from `ak-meta` rather than the crate: it is the build's own,
/// and carries the build hash the rest of the platform reports.
fn user_agent(runtime_version: &str) -> String {
    format!(
        "authentik Platform/WCP/CredProvider@{} (WebView2 {runtime_version})",
        ak_meta::full_version()
    )
}

/// The origin as a browser computes it, port included — JavaScript's
/// `location.origin`.
///
/// Deliberately not [`origin_of`], which drops the port to redact log lines.
/// This value goes into a WebView2 URI filter and is compared against what the
/// page reports, and a missing port makes both silently wrong: the filter
/// matches nothing and no error is raised anywhere.
pub(crate) fn url_origin(url: &str) -> Option<String> {
    url.parse::<tauri::Url>()
        .ok()
        .map(|parsed| parsed.origin().ascii_serialization())
}

/// Scheme and host of `url`, or a placeholder — the rest of a sign-in URL is
/// not safe to log.
pub(crate) fn origin_of(url: &str) -> String {
    url.parse::<tauri::Url>()
        .ok()
        .and_then(|parsed| {
            parsed
                .host_str()
                .map(|host| format!("{}://{host}", parsed.scheme()))
        })
        .unwrap_or_else(|| "<unparseable URL>".to_string())
}

/// Label the sign-in window is created with and looked up by.
const SIGN_IN_WINDOW: &str = "sign-in";

/// How long the window stays hidden waiting for the sign-in page, timed from
/// the moment it is built.
///
/// Has to clear WebView2's own startup or the fallback fires first and shows
/// the empty window this exists to avoid: on a warm dev machine the window
/// costs about three seconds to build and the page is up two seconds later,
/// nearly all of it WebView2 creating an environment for a fresh user-data
/// folder. Bounded anyway — an invisible window is worse than an empty one —
/// and five seconds here is still inside the ten `credprovider`'s foreground
/// nudge spends looking for this window.
const REVEAL_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5);

/// Shows the window, once, and starts the foreground ladder with it.
///
/// Built hidden so nobody sees an empty frame while WebView2 starts and the
/// page loads; there is nothing to fight the foreground for until then either.
fn reveal(
    window: &tauri::WebviewWindow,
    shown: &AtomicBool,
    ever_activated: &Arc<AtomicBool>,
    why: &str,
) {
    if shown.swap(true, Ordering::SeqCst) {
        return;
    }
    log::info!("showing the sign-in window ({why})");
    if let Err(e) = window.show() {
        log::error!("could not show the sign-in window: {e}");
    }
    if let Err(e) = window.set_focus() {
        log::debug!("could not focus the sign-in window: {e}");
    }
    match window.hwnd() {
        Ok(hwnd) => foreground::watch(hwnd.0 as isize, ever_activated.clone()),
        Err(e) => log::error!("no window handle to keep in the foreground: {e}"),
    }
}

/// Reveals the window after [`REVEAL_TIMEOUT`] whether or not a page loaded, so
/// an unreachable authentik degrades to an empty window rather than to none at
/// all. A no-op once the page-load handler has shown it — same flag.
fn reveal_after_timeout(
    app: tauri::AppHandle,
    shown: Arc<AtomicBool>,
    ever_activated: Arc<AtomicBool>,
) {
    std::thread::spawn(move || {
        std::thread::sleep(REVEAL_TIMEOUT);
        if shown.load(Ordering::SeqCst) {
            return;
        }
        match app.get_webview_window(SIGN_IN_WINDOW) {
            Some(window) => reveal(
                &window,
                &shown,
                &ever_activated,
                "the sign-in page did not load in time",
            ),
            None => log::warn!("no sign-in window to show after the reveal timeout"),
        }
    });
}

/// The handful of things a sign-in needs once it starts, which is after the
/// window already exists.
#[derive(Clone)]
struct SignInState {
    /// Origin of the sign-in URL, empty until a sign-in has been asked for —
    /// which is how the page-load handler tells the real page from the
    /// placeholder.
    origin: Arc<Mutex<String>>,
    shown: Arc<AtomicBool>,
    ever_activated: Arc<AtomicBool>,
}

/// Points the already-built window at the sign-in URL and arms the reveal.
///
/// Everything slow has already happened, so on a host preloaded at tile
/// selection this is very nearly the whole cost of a sign-in.
fn start_sign_in(app: &tauri::AppHandle, state: &SignInState, url: String, header_token: String) {
    let Some(window) = app.get_webview_window(SIGN_IN_WINDOW) else {
        log::error!("no sign-in window to start the flow in");
        signin::close(app);
        return;
    };
    if let Err(e) = url.parse::<tauri::Url>() {
        log::error!("the sign-in URL from credprovider does not parse: {e}");
        signin::close(app);
        return;
    }
    log::info!("starting the sign-in at {}", origin_of(&url));

    let Some(origin) = url_origin(&url) else {
        log::error!("the sign-in URL has no usable origin");
        signin::close(app);
        return;
    };
    *state.origin.lock().unwrap_or_else(|e| e.into_inner()) = origin.clone();
    webview2::navigate_with_header(&window, app.clone(), header_token, url, origin);
    reveal_after_timeout(
        app.clone(),
        state.shown.clone(),
        state.ever_activated.clone(),
    );
}

/// Opens the window and runs the message loop until the flow ends.
///
/// Returns a fallback report rather than sending it: `Completion::send` is
/// send-once, so a route that already answered wins over this.
fn run(
    completion: &Arc<signin::Completion>,
    control_pipe: Option<std::fs::File>,
    sign_in: Option<(String, String)>,
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

    // Separates "never took focus" from "took it and lost it again", which the
    // retry ladder reports differently when it gives up.
    let ever_activated = Arc::new(AtomicBool::new(false));
    // So the page-load handler and the timeout fallback cannot both reveal the
    // window, and both start a foreground ladder.
    let shown = Arc::new(AtomicBool::new(false));

    let setup_completion = completion.clone();
    let state = SignInState {
        origin: Arc::new(Mutex::new(String::new())),
        shown: shown.clone(),
        ever_activated: ever_activated.clone(),
    };
    let setup_state = state.clone();
    let event_completion = completion.clone();
    let data_directory = cache_path.to_path_buf();

    let ran = tauri::Builder::default()
        .setup(move |app| {
            let handle = app.handle().clone();
            let nav_completion = setup_completion.clone();
            let nav_handle = handle.clone();
            let page_state = setup_state.clone();

            // The placeholder, not the sign-in URL: pointing the builder at
            // the real one races header injection and sends the document
            // request bare. `navigate_with_header` starts it instead.
            let builder = WebviewWindowBuilder::new(
                app,
                SIGN_IN_WINDOW,
                WebviewUrl::App("index.html".into()),
            )
            .title(ak_ee_wcp_wire::WINDOW_TITLE)
            .inner_size(
                f64::from(ak_ee_wcp_wire::WINDOW_WIDTH),
                f64::from(ak_ee_wcp_wire::WINDOW_HEIGHT),
            )
            .center()
            .resizable(false)
            .minimizable(false)
            .maximizable(false)
            // The frame this process gets on the logon desktop is the
            // classic pre-Aero caption, which looks like Windows 9x next
            // to LogonUI's own chrome — not a Tauri limitation, since a
            // decorated window in an ordinary session is DWM-composited.
            // The likely cause is the restricted service-account token
            // being unable to reach DWM for that session; unverified on
            // the secure desktop, see TAURI_MIGRATION.md. Nothing is
            // lost but the close button, which escape replaces (see
            // `webview2`), as the window is fixed-size and centered.
            .decorations(false)
            // Topmost before the first paint, so the window is never
            // behind LogonUI even for a frame. Focus is a separate and
            // much less certain thing — see `foreground`.
            .always_on_top(true)
            // `reveal` shows it once there is a page to look at.
            .visible(false)
            .user_agent(&user_agent(&runtime_version))
            .data_directory(data_directory.clone())
            .incognito(true)
            .on_navigation(move |url| {
                let url = url.as_str();
                if !url.starts_with(ak_ee_wcp_wire::REDIRECT_PREFIX) {
                    log::debug!("navigating to {}", origin_of(url));
                    return true;
                }

                // Validating this URL's token needs `ak-sysd`, which
                // this process cannot reach; `credprovider` does it.
                log::info!("redirect detected; reporting it for validation");
                nav_completion.send(HostReport::Redirected {
                    url: url.to_string(),
                });
                signin::close(&nav_handle);
                // WebView2 cannot navigate to `goauthentik.io://`
                // anyway, and would say so in the window.
                false
            })
            .on_page_load(move |window, payload| {
                // Only the sign-in page counts, and a "navigation started"
                // flag does not separate it from the placeholder: `Navigate`
                // runs before the placeholder's own load completes, so such a
                // flag is already set when it arrives.
                let origin = page_state
                    .origin
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .clone();
                if !matches!(payload.event(), PageLoadEvent::Finished)
                    || origin.is_empty()
                    || url_origin(payload.url().as_str()).as_deref() != Some(origin.as_str())
                {
                    return;
                }
                reveal(
                    &window,
                    &page_state.shown,
                    &page_state.ever_activated,
                    "the sign-in page finished loading",
                );
            });

            builder.build()?;
            // The expensive part is behind us; on a preloaded host it was paid
            // while the person was still deciding to click.
            log::info!(
                "sign-in window ready, foreground_pid={:?}",
                foreground::foreground_pid()
            );

            if let Some(control_pipe) = control_pipe {
                let start_app = handle.clone();
                let start_state = setup_state.clone();
                signin::watch_control_pipe(
                    control_pipe,
                    handle.clone(),
                    setup_completion.clone(),
                    move |url, header_token| {
                        start_sign_in(&start_app, &start_state, url, header_token);
                    },
                );
            }

            // On the command line rather than over the pipe: the provider had
            // no preloaded host to reuse.
            if let Some((url, header_token)) = sign_in {
                start_sign_in(&handle, &setup_state, url, header_token);
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
                // A no-op on every route that already reported; this is for
                // the person closing the window.
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
    // Held for all of `main` so the client can still flush at exit.
    let _sentry = sentry::init(ak_meta::sentry_options("ak-browser"));

    ak_platform::log::LogBuilder::new(ak_platform::string::PlatformString::new_with_default(
        "authentik Credential Provider (Browser)",
    ))
    .with_default_filters()
    .allow_platform(true)
    .allow_stdout(false)
    .enable();

    // First line out, before anything else can fail: which commit this was
    // built from and which account it runs as.
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

    // Both optional: a host preloaded at tile selection is started later by a
    // `StartSignIn` over the control pipe.
    let sign_in = match (arg_value("--sign-in-url"), arg_value("--header-token")) {
        (Some(url), Some(header_token)) => Some((url, header_token)),
        (None, None) => None,
        _ => {
            log::error!("--sign-in-url and --header-token must be given together");
            completion.send(HostReport::Cancelled);
            return;
        }
    };

    let cache_path = browser_state_dir(Path::new(CACHE_ROOT));

    let report = run(&completion, pipes.cancel, sign_in, &cache_path);
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

    /// Cookies and local storage live in nested directories, and each run owns
    /// its directory outright, so the remove has to be recursive and take the
    /// directory with it.
    #[test]
    fn removes_a_populated_directory_entirely() {
        let dir = scratch_dir("wipe");
        std::fs::create_dir_all(dir.join("Default/Local Storage")).expect("seed nested state");
        std::fs::write(dir.join("Default/Cookies"), b"session").expect("seed a cookie jar");
        std::fs::write(dir.join("Local State"), b"{}").expect("seed a loose file");

        wipe_browser_state(&dir);

        assert!(!dir.exists(), "the directory itself should be gone");
    }

    /// Reached whenever the host exits before creating its own directory.
    #[test]
    fn tolerates_a_directory_that_is_already_gone() {
        let dir = scratch_dir("missing");
        wipe_browser_state(&dir);
        assert!(!dir.exists());
    }

    /// A WebView2 user-data folder is single-writer, so a shared one would
    /// lock out every launch after the first.
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
    /// contract. `make ee/wcp/test` does not set `AK_VERSION` the way
    /// `make ee/wcp/build` does, hence asserting the version is reported
    /// faithfully rather than that it is non-empty.
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

    /// Sign-in URLs carry tokens in the path and query.
    #[test]
    fn only_the_origin_is_logged() {
        assert_eq!(
            origin_of("https://authentik.company/if/flow/default/?token=secret"),
            "https://authentik.company"
        );
        assert_eq!(origin_of("not a url"), "<unparseable URL>");
    }
}
