// Tauri auth window launched as a child process by the credential provider DLL.
//
// It opens a webview at the auth URL and watches navigations. When the webview
// navigates to the configured redirect prefix (the OAuth2 redirect_uri in a
// real deployment), authentication is considered complete and the result is
// signalled back to the DLL over a named pipe. Closing the window without
// reaching the redirect signals cancellation.
//
// This is the cross-process equivalent of the C++ provider storing the
// authenticated username into the shared `sHookData` struct.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::fs::File;
use std::io::Write;
use std::os::windows::io::FromRawHandle;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use ak_platform::string::PlatformString;
use tauri::{WebviewUrl, WebviewWindowBuilder, WindowEvent};

/// Write a single line to the DLL's inherited anonymous pipe.
/// Protocol: `OK <username>` on success, `CANCEL` otherwise.
///
/// `handle` is the raw value of the write end that the credential provider DLL
/// passed via `--pipe-handle`; it was inherited into this process so the integer
/// is a valid handle here. We wrap it in a `File`, write the line, and let the
/// `File` drop close it (signalling EOF to the DLL's read end).
fn signal(handle: &Option<usize>, msg: &str) {
    log::info!("signalling pipe: {msg}");
    let Some(raw) = handle else {
        log::warn!("no --pipe-handle given, would have signalled: {msg}");
        return;
    };
    // SAFETY: the integer is a handle inherited from the parent for exactly this
    // purpose; taking ownership via File is correct.
    let mut f = unsafe { File::from_raw_handle(*raw as *mut std::ffi::c_void) };
    if let Err(e) = writeln!(f, "{msg}").and_then(|_| f.flush()) {
        log::error!("failed to write to inherited pipe handle: {e}");
    }
}

/// Read the value following a `--flag` in the process arguments.
fn arg(flag: &str) -> Option<String> {
    let mut it = std::env::args();
    while let Some(a) = it.next() {
        if a == flag {
            return it.next();
        }
    }
    None
}

/// The redirect URL prefix used by the authentik OAuth2 flow, matching the
/// C++ credential provider's `strKey` in `simple_handler.h`.
const REDIRECT_PREFIX: &str = "goauthentik.io://";

/// Register a WebView2 WebResourceRequested handler that injects the
/// `X-Authentik-Platform-Auth-DTH` header on every outgoing request.
///
/// This mirrors the C++ `OnBeforeResourceLoad` in `simple_handler.h` which
/// calls `request->SetHeaderByName("X-Authentik-Platform-Auth-DTH", ...)`.
/// The closure is dispatched asynchronously to the main thread by Tauri.
#[cfg(target_os = "windows")]
fn inject_header_token(window: &tauri::WebviewWindow, header_token: String) {
    use webview2_com::{
        Microsoft::Web::WebView2::Win32::{
            ICoreWebView2WebResourceRequest, COREWEBVIEW2_WEB_RESOURCE_CONTEXT_ALL,
        },
        WebResourceRequestedEventHandler,
    };
    use windows_core::HSTRING;

    log::debug!("registering WebResourceRequested filter for header injection");

    if let Err(e) = window.with_webview(move |wv| {
        let controller = wv.controller();
        let wv2 = match unsafe { controller.CoreWebView2() } {
            Ok(v) => v,
            Err(e) => {
                log::error!("CoreWebView2 failed: {e}");
                return;
            }
        };

        if let Err(e) = unsafe {
            wv2.AddWebResourceRequestedFilter(
                &HSTRING::from("*"),
                COREWEBVIEW2_WEB_RESOURCE_CONTEXT_ALL,
            )
        } {
            log::error!("AddWebResourceRequestedFilter failed: {e}");
            return;
        }

        log::debug!("WebResourceRequested filter registered; injecting header on all requests");

        let mut token = 0i64;
        if let Err(e) = unsafe {
            wv2.add_WebResourceRequested(
                &WebResourceRequestedEventHandler::create(Box::new(move |_sender, args| {
                    let Some(args) = args else {
                        return Ok(());
                    };
                    let req: ICoreWebView2WebResourceRequest = args.Request()?;
                    let hdrs = req.Headers()?;
                    hdrs.SetHeader(
                        &HSTRING::from("X-Authentik-Platform-Auth-DTH"),
                        &HSTRING::from(header_token.as_str()),
                    )?;
                    Ok(())
                })),
                &mut token,
            )
        } {
            log::error!("add_WebResourceRequested failed: {e}");
        }
    }) {
        log::error!("with_webview dispatch failed: {e}");
    }
}

fn main() {
    ak_platform::log::init_log(PlatformString::new_with_default("authentik WCP Browser"));

    let pipe = arg("--pipe-handle").and_then(|s| s.parse::<usize>().ok());

    log::info!(
        "auth-app starting (pid={}, pipe-handle={:?})",
        std::process::id(),
        pipe
    );

    // Tracks whether we already signalled success, so the window-close handler
    // doesn't also send a spurious CANCEL.
    let completed = Arc::new(AtomicBool::new(false));

    let pipe_close = pipe;
    let completed_close = completed.clone();

    tauri::Builder::default()
        .setup(move |app| {
            // Mirror C++ simple_app.cc: call auth_start_async to get the real
            // authentik OAuth2 URL from the backend before opening the webview.
            let (url, header_token) = match ak_ffi::ffi::sys_auth_start_async() {
                Ok(r) => r,
                Err(e) => {
                    log::error!("auth_start_async failed: {e}");
                    signal(&pipe, "CANCEL");
                    std::process::exit(1);
                }
            };

            // Log the origin only (scheme + host) — the path may contain tokens.
            if let Ok(parsed) = url.parse::<tauri::Url>() {
                log::info!(
                    "auth URL: {}://{}",
                    parsed.scheme(),
                    parsed.host_str().unwrap_or("<no host>")
                );
            }

            let target: tauri::Url = match url.parse() {
                Ok(u) => u,
                Err(e) => {
                    log::error!("invalid auth URL from backend: {e}");
                    signal(&pipe, "CANCEL");
                    std::process::exit(1);
                }
            };
            let pipe_nav = pipe;
            let completed_nav = completed.clone();

            log::info!("opening webview");
            let window = WebviewWindowBuilder::new(app, "auth", WebviewUrl::External(target))
                .title("Authenticate")
                .inner_size(900.0, 700.0)
                .center()
                .on_navigation(move |u| {
                    let url_str = u.as_str();
                    if url_str.starts_with(REDIRECT_PREFIX) {
                        log::info!("redirect detected (prefix match), calling sys_auth_url");
                        // Mirror C++ simple_handler.h OnBeforeResourceLoad: validate
                        // the token embedded in the redirect URL and extract the
                        // authenticated username.
                        let msg = match ak_ffi::ffi::sys_auth_url(url_str) {
                            Ok(Some(username)) => {
                                log::info!("sys_auth_url: authenticated as '{username}'");
                                format!("OK {username}")
                            }
                            Ok(None) => {
                                log::warn!("sys_auth_url: token validation failed");
                                "CANCEL".to_string()
                            }
                            Err(e) => {
                                log::error!("sys_auth_url error: {e}");
                                "CANCEL".to_string()
                            }
                        };
                        signal(&pipe_nav, &msg);
                        completed_nav.store(true, Ordering::SeqCst);
                        std::process::exit(0);
                    }
                    log::debug!("navigation: {}", u.host_str().unwrap_or(url_str));
                    true
                })
                .build()?;

            log::info!("webview window built");

            // Mirror C++ OnBeforeResourceLoad: inject X-Authentik-Platform-Auth-DTH
            // on every outgoing request via WebView2's WebResourceRequested event.
            #[cfg(target_os = "windows")]
            inject_header_token(&window, header_token);

            Ok(())
        })
        .on_window_event(move |_window, event| {
            if let WindowEvent::CloseRequested { .. } = event {
                if !completed_close.load(Ordering::SeqCst) {
                    log::info!("window closed without completing auth — sending CANCEL");
                    signal(&pipe_close, "CANCEL");
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running auth-app");
}
