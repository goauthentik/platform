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

fn main() {
    let url = arg("--url").unwrap_or_else(|| "https://example.com".to_string());
    let redirect = arg("--redirect").unwrap_or_else(|| "https://www.iana.org".to_string());
    let pipe = arg("--pipe-handle").and_then(|s| s.parse::<usize>().ok());

    // Tracks whether we already signalled success, so the window-close handler
    // doesn't also send a spurious CANCEL.
    let completed = Arc::new(AtomicBool::new(false));

    let pipe_close = pipe;
    let completed_close = completed.clone();

    tauri::Builder::default()
        .setup(move |app| {
            let target: tauri::Url = url.parse().expect("invalid --url");
            let pipe_nav = pipe;
            let completed_nav = completed.clone();
            let redirect = redirect.clone();

            WebviewWindowBuilder::new(app, "auth", WebviewUrl::External(target))
                .title("Authenticate")
                .inner_size(900.0, 700.0)
                .center()
                .on_navigation(move |u| {
                    if u.as_str().starts_with(&redirect) {
                        // Auth complete: signal the username (the redirect URL
                        // stands in for it here) back to the DLL and exit.
                        signal(&pipe_nav, &format!("OK {u}"));
                        completed_nav.store(true, Ordering::SeqCst);
                        std::process::exit(0);
                    }
                    true
                })
                .build()?;
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
