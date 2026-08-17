//! The two places this host touches WebView2 directly: finding out whether the
//! runtime is installed at all, and injecting the interactive-auth header on
//! every request the sign-in window makes.

use webview2_com::{
    Microsoft::Web::WebView2::Win32::{
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

/// Registers a `WebResourceRequested` handler that sets the interactive-auth
/// header on every outgoing request.
///
/// This is the WebView2 equivalent of the CEF host's `on_before_resource_load`
/// calling `set_header_by_name`. The closure is dispatched to the UI thread by
/// Tauri, so this returns before the handler is actually installed.
///
/// Every failure is logged rather than fatal: a window that loads without the
/// header gets a comprehensible error from authentik, where a dead process
/// gets the user a bare cancellation.
pub fn inject_header(window: &tauri::WebviewWindow, header_token: String) {
    log::debug!("registering the WebResourceRequested filter for header injection");

    if let Err(e) = window.with_webview(move |webview| {
        let core = match unsafe { webview.controller().CoreWebView2() } {
            Ok(core) => core,
            Err(e) => {
                log::error!("CoreWebView2 failed; requests will go out unauthenticated: {e}");
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
            return;
        }

        log::debug!("header injection registered on all requests");
    }) {
        log::error!("could not reach the webview to register header injection: {e}");
    }
}
