//! CEF app/browser-process handler: on context init, fetches the sign-in
//! URL and opens the browser window.

use cef::*;

use crate::handler::{SignInClient, SignInHandler, file_from_raw_handle};
use crate::window::SignInWindowDelegate;

wrap_app! {
    pub struct HostApp {
        result_pipe: usize,
        cancel_pipe: Option<usize>,
    }

    impl App {
        fn browser_process_handler(&self) -> Option<BrowserProcessHandler> {
            Some(HostBrowserProcessHandler::new(self.result_pipe, self.cancel_pipe))
        }
    }
}

wrap_browser_process_handler! {
    struct HostBrowserProcessHandler {
        result_pipe: usize,
        cancel_pipe: Option<usize>,
    }

    impl BrowserProcessHandler {
        // `on_context_initialized` runs on the `initialize` call stack, before
        // `run_message_loop` — the crash stack has `cef_initialize` several
        // frames below the window creation. The C++ never built the browser
        // there: it only flagged the context as ready and created the window
        // later, once the loop was pumping. Post the work instead of doing it
        // re-entrantly, which also keeps a blocking gRPC call out of
        // `CefInitialize`.
        fn on_context_initialized(&self) {
            let mut task = OpenSignInWindow::new(self.result_pipe, self.cancel_pipe);
            post_task(ThreadId::UI, Some(&mut task));
        }
    }
}

wrap_task! {
    struct OpenSignInWindow {
        result_pipe: usize,
        cancel_pipe: Option<usize>,
    }

    impl Task {
        fn execute(&self) {
            open_sign_in_window(self.result_pipe, self.cancel_pipe);
        }
    }
}

fn open_sign_in_window(result_pipe: usize, cancel_pipe: Option<usize>) {
    let start = match crate::sysd::sys_auth_start_async() {
        Ok(s) => s,
        Err(e) => {
            log::error!("sys_auth_start_async failed: {e}");
            let mut pipe = file_from_raw_handle(result_pipe);
            let _ = wire::write_auth_result(
                &mut pipe,
                &wire::AuthResult::Failed { reason: e.to_string() },
            );
            quit_message_loop();
            return;
        }
    };

    let result_pipe = file_from_raw_handle(result_pipe);
    let cancel_pipe = cancel_pipe.map(file_from_raw_handle);
    let inner = SignInHandler::new(start.header_token, result_pipe, cancel_pipe);
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
    let mut delegate = SignInWindowDelegate::new(std::cell::RefCell::new(browser_view), start.url);
    window_create_top_level(Some(&mut delegate));
    log::info!("sign-in window requested");
}
