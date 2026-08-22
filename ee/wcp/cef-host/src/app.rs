//! CEF app/browser-process handler: on context init, opens the sign-in
//! window at the URL `credprovider` already resolved before spawning this
//! process.

use std::fs::File;
use std::os::windows::io::FromRawHandle;

use cef::*;
use windows::Win32::System::Console::{GetStdHandle, STD_INPUT_HANDLE, STD_OUTPUT_HANDLE};

use crate::handler::{SignInClient, SignInHandler};
use crate::window::SignInWindowDelegate;

wrap_app! {
    pub struct HostApp {
        sign_in_url: String,
        header_token: String,
    }

    impl App {
        fn browser_process_handler(&self) -> Option<BrowserProcessHandler> {
            Some(HostBrowserProcessHandler::new(
                self.sign_in_url.clone(),
                self.header_token.clone(),
            ))
        }
    }
}

wrap_browser_process_handler! {
    struct HostBrowserProcessHandler {
        sign_in_url: String,
        header_token: String,
    }

    impl BrowserProcessHandler {
        // `on_context_initialized` runs on the `initialize` call stack, before
        // `run_message_loop` — the crash stack has `cef_initialize` several
        // frames below the window creation. The C++ never built the browser
        // there: it only flagged the context as ready and created the window
        // later, once the loop was pumping. Post the work instead of doing it
        // re-entrantly.
        fn on_context_initialized(&self) {
            let mut task = OpenSignInWindow::new(self.sign_in_url.clone(), self.header_token.clone());
            post_task(ThreadId::UI, Some(&mut task));
        }
    }
}

wrap_task! {
    struct OpenSignInWindow {
        sign_in_url: String,
        header_token: String,
    }

    impl Task {
        fn execute(&self) {
            open_sign_in_window(self.sign_in_url.clone(), self.header_token.clone());
        }
    }
}

fn open_sign_in_window(sign_in_url: String, header_token: String) {
    // Both handles were already open and access-checked in `credprovider`
    // (running as SYSTEM on the real logon scenarios) before this process
    // even existed — inherited via `STARTUPINFOW`, not opened by this
    // process's own (low-privilege) token (`BROWSER_PRIVILEGE.md`'s "Roads
    // not taken").
    let result_pipe = match unsafe { GetStdHandle(STD_OUTPUT_HANDLE) } {
        Ok(h) => unsafe { File::from_raw_handle(h.0) },
        Err(e) => {
            log::error!("could not get the inherited result pipe: {e}");
            quit_message_loop();
            return;
        }
    };
    // Best-effort: no way left to report a result at all if this fails, but
    // the sign-in itself does not depend on cancellation working.
    let cancel_pipe = match unsafe { GetStdHandle(STD_INPUT_HANDLE) } {
        Ok(h) => Some(unsafe { File::from_raw_handle(h.0) }),
        Err(e) => {
            log::error!("could not get the inherited cancel pipe: {e}");
            None
        }
    };
    let inner = SignInHandler::new(header_token, result_pipe, cancel_pipe);
    let mut client = SignInClient::new(inner);

    let browser_settings = BrowserSettings::default();
    log::info!("creating the browser view");
    // Created with no URL, as the C++ did. Passing the sign-in URL here starts
    // the navigation from inside `add_child_view`, which is where the browser
    // itself gets created; the window delegate loads it once that has finished.
    let browser_view = browser_view_create(
        Some(&mut client),
        None,
        Some(&browser_settings),
        None,
        None,
        None,
    );
    // A `None` here leaves the window empty and never shown, which the
    // credential provider only ever sees as a bare cancellation.
    if browser_view.is_none() {
        log::error!("browser_view_create returned no view");
    }

    log::info!("creating the top-level window");
    let mut delegate = SignInWindowDelegate::new(
        std::cell::RefCell::new(browser_view),
        sign_in_url,
        std::rc::Rc::new(std::cell::Cell::new(false)),
    );
    window_create_top_level(Some(&mut delegate));
    log::info!("sign-in window requested");
}
