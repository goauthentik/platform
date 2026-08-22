//! The two places this host touches WebView2 directly: finding out whether the
//! runtime is installed at all, and injecting the interactive-auth header on
//! every request the sign-in window makes.

use webview2_com::{
    AcceleratorKeyPressedEventHandler,
    Microsoft::Web::WebView2::Win32::{
        COREWEBVIEW2_KEY_EVENT_KIND, COREWEBVIEW2_KEY_EVENT_KIND_KEY_DOWN,
        COREWEBVIEW2_WEB_RESOURCE_CONTEXT_ALL, ICoreWebView2WebResourceRequest,
    },
    WebResourceRequestedEventHandler,
};
use windows_core::HSTRING;
use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE};

/// The Evergreen runtime's update client, the documented way to detect it. Its
/// `pv` value is the installed runtime version.
const RUNTIME_CLIENT_KEY: &str =
    r"SOFTWARE\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}";

/// Same key as seen by a 32-bit installer on 64-bit Windows, which is where a
/// machine-wide install actually lands.
const RUNTIME_CLIENT_KEY_WOW64: &str =
    r"SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}";

/// Installed Evergreen runtime version, or `None` if the runtime is missing.
///
/// Checked before the window is built purely so the *log* says which of the
/// many ways to reach a cancellation this was. The person signing in still
/// sees a bare "Login attempt cancelled": `HostReport` carries no failure
/// reason, deliberately, since the host is not the side that decides whether a
/// sign-in succeeded. Without this check the event log would be just as silent
/// as the CEF host's `0x80000003` exits were.
///
/// The `HKCU` fallback is checked last and is close to useless here: this
/// process runs as the `ak-wcp-browser` service account, so a per-user install
/// belonging to the person signing in is in a hive we cannot see. It costs one
/// registry read and covers running the host by hand as an ordinary user.
pub fn runtime_version() -> Option<String> {
    let candidates = [
        (HKEY_LOCAL_MACHINE, RUNTIME_CLIENT_KEY_WOW64),
        (HKEY_LOCAL_MACHINE, RUNTIME_CLIENT_KEY),
        (HKEY_CURRENT_USER, RUNTIME_CLIENT_KEY),
    ];

    candidates.into_iter().find_map(|(root, path)| {
        winreg::RegKey::predef(root)
            .open_subkey(path)
            .and_then(|key| key.get_value::<String, _>("pv"))
            .ok()
            .filter(|version| !version.is_empty() && version != "0.0.0.0")
    })
}

/// Registers the header-injection handler and *then* starts the sign-in
/// navigation, in that order, on the webview's own thread.
///
/// The order is the whole point. `with_webview` dispatches its closure to the
/// main thread rather than running it inline, so anything the webview was
/// already loading is in flight before the handler exists. Building the window
/// straight onto the sign-in URL therefore raced — and lost: the document
/// request went out bare, and the only request the flow makes carried no
/// header at all. The CEF host had the same hazard and answered it the same
/// way, creating the browser with no URL and loading it once the client was
/// attached (`browser_view_create(.., None, ..)` and the window delegate's
/// `load_url`).
///
/// So the window is built on the bundled placeholder page and the real
/// navigation happens here, after `add_WebResourceRequested` has returned.
///
/// Failing closed: if any step fails there is no way to authenticate the
/// requests, so the window is closed rather than pointed at authentik without
/// the header, which would only fail further along and less legibly.
pub fn navigate_with_header(
    window: &tauri::WebviewWindow,
    app: tauri::AppHandle,
    header_token: String,
    sign_in_url: String,
) {
    let on_failure = app.clone();
    if let Err(e) = window.with_webview(move |webview| {
        let core = match unsafe { webview.controller().CoreWebView2() } {
            Ok(core) => core,
            Err(e) => {
                log::error!("CoreWebView2 failed; cannot authenticate the sign-in requests: {e}");
                crate::signin::close(&app);
                return;
            }
        };

        if let Err(e) = unsafe {
            core.AddWebResourceRequestedFilter(
                &HSTRING::from("*"),
                COREWEBVIEW2_WEB_RESOURCE_CONTEXT_ALL,
            )
        } {
            log::error!("AddWebResourceRequestedFilter failed: {e}");
            crate::signin::close(&app);
            return;
        }

        let header = HSTRING::from(ak_ee_wcp_wire::AUTH_HEADER_NAME);
        let token = HSTRING::from(header_token.as_str());
        let mut registration = 0i64;
        if let Err(e) = unsafe {
            core.add_WebResourceRequested(
                &WebResourceRequestedEventHandler::create(Box::new(move |_sender, args| {
                    let Some(args) = args else {
                        return Ok(());
                    };
                    let request: ICoreWebView2WebResourceRequest = args.Request()?;
                    request.Headers()?.SetHeader(&header, &token)?;
                    Ok(())
                })),
                &mut registration,
            )
        } {
            log::error!("add_WebResourceRequested failed: {e}");
            crate::signin::close(&app);
            return;
        }

        // Escape is the other way out of a frameless window, and the one
        // that still works if the injected cancel button never makes it into
        // the page. Registered on the controller rather than the page, so no
        // amount of the page's own key handling can swallow it.
        let escape_app = app.clone();
        let mut escape_registration = 0i64;
        if let Err(e) = unsafe {
            webview.controller().add_AcceleratorKeyPressed(
                &AcceleratorKeyPressedEventHandler::create(Box::new(move |_sender, args| {
                    let Some(args) = args else {
                        return Ok(());
                    };
                    const VK_ESCAPE: u32 = 0x1B;
                    let mut kind = COREWEBVIEW2_KEY_EVENT_KIND::default();
                    args.KeyEventKind(&mut kind)?;
                    let mut key = 0u32;
                    args.VirtualKey(&mut key)?;
                    if kind == COREWEBVIEW2_KEY_EVENT_KIND_KEY_DOWN && key == VK_ESCAPE {
                        log::info!("escape pressed; cancelling the sign-in");
                        args.SetHandled(true)?;
                        crate::signin::close(&escape_app);
                    }
                    Ok(())
                })),
                &mut escape_registration,
            )
        } {
            log::warn!("could not register the escape-to-cancel handler: {e}");
        }

        log::debug!("header injection registered; starting the sign-in navigation");
        if let Err(e) = unsafe { core.Navigate(&HSTRING::from(sign_in_url.as_str())) } {
            log::error!("could not navigate to the sign-in URL: {e}");
            crate::signin::close(&app);
        }
    }) {
        log::error!("could not reach the webview to register header injection: {e}");
        crate::signin::close(&on_failure);
    }
}
