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
mod identity;
mod signin;
mod webview2;

use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use ak_ee_wcp_wire::HostReport;
use tauri::webview::PageLoadEvent;
use tauri::{Manager, WebviewUrl, WebviewWindowBuilder, WindowEvent};

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

/// The URL's origin exactly as a browser computes it — `scheme://host[:port]`,
/// with a default port omitted — matching JavaScript's `location.origin`.
///
/// Deliberately not [`origin_of`], which exists to redact log lines and drops
/// the port. This value is compared against what the page reports and is
/// pasted into a WebView2 URI filter, so dropping the port makes both silently
/// wrong: the filter matches nothing and the script's own origin check never
/// fires. That is exactly what happened when this reused `origin_of`.
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

/// Host-internal URL the injected cancel button navigates to.
///
/// Its own scheme rather than a path under `REDIRECT_PREFIX`: WebView2 cannot
/// navigate either of them, but a separate scheme cannot be confused with a
/// real `goauthentik.io://` callback no matter what authentik puts in one.
pub(crate) const CANCEL_URL: &str = "akwcp://cancel";

/// `s` as a JavaScript string literal.
///
/// The values this wraps are a URL origin and an opaque token from `ak-sysd`;
/// neither is ours to assume anything about, and both end up inside a script
/// that runs on the sign-in page.
fn js_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '\"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\u{2028}' => out.push_str("\\u2028"),
            '\u{2029}' => out.push_str("\\u2029"),
            // `</script>` cannot terminate anything here — this is never
            // inline in a document — but `<` is cheap to neutralise.
            '<' => out.push_str("\\u003c"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// The script that runs on every document the sign-in window loads.
///
/// Two jobs, both hanging off `window.fetch`:
///
/// **A cancel link.** The window is frameless (see the builder below), so
/// there is no system close button, and LogonUI's own cancel sits behind a
/// topmost window. `/api/v3/core/brands/current/` carries `ui_footer_links`,
/// which the flow executor renders in its footer, so appending one entry there
/// gets a cancel link that is authentik's own component in authentik's own
/// styling — nothing here has an opinion about how it looks. Escape does the
/// same job without any of this and is what GCPW relies on; this is the
/// visible affordance on top.
///
/// **The interactive-auth header**, on `fetch` calls back to authentik. The
/// host already sets it at the WebView2 layer for every request, which is
/// where it belongs; this repeats it for the requests that layer does not see,
/// a service worker's being the ones that matter.
///
/// Scoped to `origin` on purpose. A token that only ever needs to reach
/// authentik should not be added to a request the page makes somewhere else,
/// and the flow can hand off to an upstream IdP mid-sign-in.
///
/// `token` reaching page script at all is a real cost: at the WebView2 layer
/// it stays in this process, and here it does not. It is passed as an argument
/// rather than written into the function body so that reading `fetch.toString`
/// does not hand it over, but anything running in that document is closer to
/// it than it was. See `TAURI_MIGRATION.md`.
fn page_script(origin: &str, token: &str) -> String {
    format!(
        r#"
(function (ORIGIN, HEADER, TOKEN, CANCEL) {{
  if (window.__akWcpInstalled) return;
  window.__akWcpInstalled = true;

  var BRAND = '/api/v3/core/brands/current/';
  var inner = window.fetch;
  if (typeof inner !== 'function') return;

  function toAuthentik(url) {{
    try {{
      return new URL(url, document.baseURI).origin === ORIGIN;
    }} catch (e) {{
      return false;
    }}
  }}

  window.fetch = function (input, init) {{
    var url = typeof input === 'string' ? input : (input && input.url) || '';
    var args = arguments;
    if (toAuthentik(url)) {{
      try {{
        var request = new Request(input, init);
        request.headers.set(HEADER, TOKEN);
        args = [request];
      }} catch (e) {{
        // A request we cannot copy is better sent without the header than
        // not sent at all: the WebView2 layer still adds it.
      }}
    }}

    var pending = inner.apply(this, args);
    if (url.indexOf(BRAND) === -1) return pending;

    return pending.then(function (response) {{
      if (!response || !response.ok) return response;
      return response.clone().json().then(function (body) {{
        if (!body || !Array.isArray(body.ui_footer_links)) return response;
        var already = body.ui_footer_links.some(function (l) {{
          return l && l.href === CANCEL;
        }});
        if (already) return response;
        body.ui_footer_links = body.ui_footer_links.concat([
          {{ name: 'Cancel sign-in', href: CANCEL }}
        ]);
        var headers = new Headers(response.headers);
        headers.set('content-type', 'application/json');
        return new Response(JSON.stringify(body), {{
          status: response.status,
          statusText: response.statusText,
          headers: headers
        }});
      }}).catch(function () {{
        // A brand payload we cannot read is not worth failing the page for.
        return response;
      }});
    }});
  }};
}})({origin}, {header}, {token}, {cancel});
"#,
        origin = js_string(origin),
        header = js_string(ak_ee_wcp_wire::AUTH_HEADER_NAME),
        token = js_string(token),
        cancel = js_string(CANCEL_URL),
    )
}

/// Label the sign-in window is created with and looked up by.
const SIGN_IN_WINDOW: &str = "sign-in";

/// How long the window stays hidden waiting for the sign-in page, timed from
/// the moment it is built.
///
/// Has to clear WebView2's own startup or the fallback fires first and shows
/// the empty window this exists to avoid. Measured on a warm dev machine:
/// building the window costs about three seconds on its own, and the sign-in
/// page is up about two seconds after that. Delaying the page itself by two
/// seconds barely moves the total, so the cost is WebView2 creating an
/// environment rather than anything on the network — every run gets a fresh
/// user-data folder (`browser_state_dir`), so every run pays first-run
/// initialisation.
///
/// Bounded all the same, because an invisible window is worse than an empty
/// one: if authentik is unreachable the person at the logon screen would
/// otherwise be left with nothing at all and no sign a sign-in was attempted.
/// Five seconds here is about eight from the spawn, still inside the ten
/// `credprovider`'s foreground nudge spends looking for this window.
const REVEAL_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5);

/// Shows the window, once, and starts the foreground ladder with it.
///
/// The window is built hidden and revealed here rather than shown immediately,
/// so nobody sees an empty frame while WebView2 starts up and the sign-in page
/// loads. There is nothing to fight the foreground for until then either, so
/// the ladder starts from here too.
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

/// Reveals the window after [`REVEAL_TIMEOUT`] whether or not a page ever
/// loaded, so a slow or unreachable authentik degrades to the empty window
/// this is trying to avoid rather than to no window at all.
///
/// A no-op once the page-load handler has already shown it — `reveal` is
/// guarded by the same flag.
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

/// The handful of things a sign-in needs to reach once it starts, which is
/// after the window already exists.
#[derive(Clone)]
struct SignInState {
    /// Origin of the sign-in URL, empty until a sign-in has been asked for.
    /// The page-load handler compares against it to tell the real page from
    /// the placeholder, and an empty string matches neither.
    origin: Arc<Mutex<String>>,
    shown: Arc<AtomicBool>,
    ever_activated: Arc<AtomicBool>,
}

/// Points the already-built window at the sign-in URL and arms the reveal.
///
/// Everything slow has happened before this runs — the window exists and
/// WebView2's environment is up — so when the host was preloaded at tile
/// selection, this is very nearly the whole cost of a sign-in.
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
    let script = page_script(&origin, &header_token);
    webview2::navigate_with_header(&window, app.clone(), header_token, url, origin, script);
    reveal_after_timeout(
        app.clone(),
        state.shown.clone(),
        state.ever_activated.clone(),
    );
}

/// Opens the window and runs the message loop until the flow ends.
///
/// Returns the report to fall back on rather than sending it: `main` sends
/// whatever comes back, and `Completion::send` is send-once, so a route that
/// already answered — a redirect, a cancellation — wins over this.
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

    // Whether the window has ever held the foreground, which separates "never
    // took focus" from "took it and lost it again". Fed by the `Focused`
    // events below and read by the retry ladder when it gives up.
    let ever_activated = Arc::new(AtomicBool::new(false));
    // Whether the window has been revealed, so the page-load handler and the
    // timeout fallback cannot both show it (and both start a foreground ladder).
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

            // Built on the bundled placeholder rather than the sign-in URL:
            // the real navigation is started by `navigate_with_header` once
            // header injection is actually registered. See its doc comment —
            // pointing the builder at the sign-in URL races the handler and
            // sends the document request bare.
            let builder = WebviewWindowBuilder::new(
                app,
                SIGN_IN_WINDOW,
                WebviewUrl::App("index.html".into()),
            )
            .title("Sign in with authentik")
            .inner_size(
                f64::from(ak_ee_wcp_wire::WINDOW_WIDTH),
                f64::from(ak_ee_wcp_wire::WINDOW_HEIGHT),
            )
            .center()
            .resizable(false)
            .minimizable(false)
            .maximizable(false)
            // No frame at all, because the frame this process gets on
            // the logon desktop is the classic pre-Aero caption, which
            // looks like Windows 9x next to LogonUI's own chrome.
            //
            // Not a Tauri limitation: measured in an ordinary session,
            // a decorated window here gets a proper DWM-composited
            // frame (`DwmGetWindowAttribute`'s extended frame bounds
            // come back inset from the window rect, which only happens
            // when DWM is drawing it). The CEF window had modern chrome
            // too, until it stopped running as SYSTEM. What changed is
            // the token, so the likely cause is that the restricted
            // service-account token cannot reach DWM for that session
            // and the window falls back to legacy non-client painting.
            // Unverified on the secure desktop — see TAURI_MIGRATION.md.
            //
            // The window is fixed-size and centered, so there is nothing
            // to drag or resize. Cancelling is the one thing the frame
            // did carry: LogonUI's own cancel is behind a topmost window
            // and the system close button goes with the caption, which
            // left a sign-in with no way out short of the credential
            // provider giving up. `page_script` and the escape
            // handler in `webview2` replace it.
            .decorations(false)
            // Topmost before the first paint, so the window is never
            // behind LogonUI even for a frame. Asking for focus is a
            // separate and much less certain thing — see `foreground`.
            .always_on_top(true)
            // Built hidden; `reveal` shows it once there is a page to
            // look at. Shown immediately, it is an empty frame for the
            // second or so WebView2 takes to start and load.
            .visible(false)
            .user_agent(&user_agent(&runtime_version))
            .data_directory(data_directory.clone())
            .incognito(true)
            .on_navigation(move |url| {
                let url = url.as_str();
                if url == CANCEL_URL {
                    log::info!("cancelled from the sign-in window");
                    nav_completion.send(HostReport::Cancelled);
                    signin::close(&nav_handle);
                    return false;
                }
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
            })
            .on_page_load(move |window, payload| {
                // Only the sign-in page counts. The placeholder this window
                // is built on finishes loading too, and a flag saying "the
                // real navigation has started" does not separate them:
                // `Navigate` is called from the event loop before the
                // placeholder's own load completes, so such a flag is already
                // set when it arrives — which revealed an empty window every
                // time.
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
            // The expensive part is behind us: the window exists and WebView2
            // has an environment. When the credential provider preloaded this
            // process at tile selection, that cost was paid while the person
            // was still deciding to click.
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

            // Handed the sign-in on the command line rather than over the
            // pipe: the provider had no preloaded host to reuse and spawned
            // one on the spot.
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

    // Both optional. A host preloaded when the tile was selected is started
    // later, by a `StartSignIn` over the control pipe; only a host spawned at
    // submit time is handed them here.
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

    /// The token goes into a script as a literal, so anything that could end
    /// that literal early would put an `ak-sysd` token where the page can read
    /// it — or break the script and take the cancel link with it.
    #[test]
    fn js_strings_cannot_escape_their_own_quotes() {
        assert_eq!(js_string("plain"), "\"plain\"");
        assert_eq!(js_string("a\"b"), "\"a\\\"b\"");
        assert_eq!(js_string("a\\b"), "\"a\\\\b\"");
        assert_eq!(js_string("</script>"), "\"\\u003c/script>\"");
    }

    /// The whole script is generated, so a malformed origin or token has to
    /// come out as data rather than as code.
    #[test]
    fn the_page_script_quotes_what_it_is_given() {
        let token = "tok\");alert(1);//";
        let script = page_script("https://authentik.company", token);
        assert!(
            script.contains(&js_string(token)),
            "the token should appear escaped: {script}"
        );
        assert!(
            !script.contains(token),
            "the raw token would close the literal and run as code"
        );
        // The header name and the cancel URL come from the constants rather
        // than being written out a second time in JavaScript.
        assert!(script.contains(&js_string(ak_ee_wcp_wire::AUTH_HEADER_NAME)));
        assert!(script.contains(&js_string(CANCEL_URL)));
        assert!(script.contains(&js_string("https://authentik.company")));
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
