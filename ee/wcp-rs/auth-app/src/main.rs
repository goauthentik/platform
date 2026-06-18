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

use tauri::{WebviewUrl, WebviewWindowBuilder, WindowEvent};

/// Write a single line to the DLL's inherited anonymous pipe.
/// Protocol: `OK <username>` on success, `CANCEL` otherwise.
///
/// `handle` is the raw value of the write end that the credential provider DLL
/// passed via `--pipe-handle`; it was inherited into this process so the integer
/// is a valid handle here. We wrap it in a `File`, write the line, and let the
/// `File` drop close it (signalling EOF to the DLL's read end).
fn signal(handle: &Option<usize>, msg: &str) {
    let Some(raw) = handle else {
        eprintln!("auth-app: no --pipe-handle given, would have signalled: {msg}");
        return;
    };
    // SAFETY: the integer is a handle inherited from the parent for exactly this
    // purpose; taking ownership via File is correct.
    let mut f = unsafe { File::from_raw_handle(*raw as *mut std::ffi::c_void) };
    if let Err(e) = writeln!(f, "{msg}").and_then(|_| f.flush()) {
        eprintln!("auth-app: failed to write to inherited pipe handle: {e}");
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

    if let Err(e) = window.with_webview(move |wv| {
        let controller = wv.controller();
        let wv2 = match unsafe { controller.CoreWebView2() } {
            Ok(v) => v,
            Err(e) => {
                eprintln!("auth-app: CoreWebView2 failed: {e}");
                return;
            }
        };

        if let Err(e) = unsafe {
            wv2.AddWebResourceRequestedFilter(
                &HSTRING::from("*"),
                COREWEBVIEW2_WEB_RESOURCE_CONTEXT_ALL,
            )
        } {
            eprintln!("auth-app: AddWebResourceRequestedFilter failed: {e}");
            return;
        }

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
            eprintln!("auth-app: add_WebResourceRequested failed: {e}");
        }
    }) {
        eprintln!("auth-app: with_webview dispatch failed: {e}");
    }
}

fn main() {
    let pipe = arg("--pipe-handle").and_then(|s| s.parse::<usize>().ok());

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
                    eprintln!("auth_start_async failed: {e}");
                    signal(&pipe, "CANCEL");
                    std::process::exit(1);
                }
            };

            let target: tauri::Url = match url.parse() {
                Ok(u) => u,
                Err(e) => {
                    eprintln!("invalid auth URL from backend: {e}");
                    signal(&pipe, "CANCEL");
                    std::process::exit(1);
                }
            };
            let pipe_nav = pipe;
            let completed_nav = completed.clone();

            let window = WebviewWindowBuilder::new(app, "auth", WebviewUrl::External(target))
                .title("Authenticate")
                .inner_size(900.0, 700.0)
                .center()
                .on_navigation(move |u| {
                    if u.as_str().starts_with(REDIRECT_PREFIX) {
                        // Mirror C++ simple_handler.h OnBeforeResourceLoad: validate
                        // the token embedded in the redirect URL and extract the
                        // authenticated username.
                        let msg = match ak_ffi::ffi::sys_auth_url(u.as_str()) {
                            Ok(Some(username)) => format!("OK {username}"),
                            Ok(None) => {
                                eprintln!("auth_url: token validation failed");
                                "CANCEL".to_string()
                            }
                            Err(e) => {
                                eprintln!("auth_url error: {e}");
                                "CANCEL".to_string()
                            }
                        };
                        signal(&pipe_nav, &msg);
                        completed_nav.store(true, Ordering::SeqCst);
                        std::process::exit(0);
                    }
                    true
                })
                .build()?;

            // Mirror C++ OnBeforeResourceLoad: inject X-Authentik-Platform-Auth-DTH
            // on every outgoing request via WebView2's WebResourceRequested event.
            #[cfg(target_os = "windows")]
            inject_header_token(&window, header_token);

            Ok(())
        })
        .on_window_event(move |_window, event| {
            if let WindowEvent::CloseRequested { .. } = event {
                if !completed_close.load(Ordering::SeqCst) {
                    signal(&pipe_close, "CANCEL");
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running auth-app");
}
