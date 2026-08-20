//! CEF app/browser-process handler: on context init, opens the sign-in
//! window at the URL `credprovider` already resolved before spawning this
//! process.

use cef::*;

use crate::handler::{SignInClient, SignInHandler, connect_cancel_pipe, connect_result_pipe};
use crate::window::SignInWindowDelegate;

wrap_app! {
    pub struct HostApp {
        result_pipe: String,
        cancel_pipe: Option<String>,
        sign_in_url: String,
        header_token: String,
    }

    impl App {
        fn browser_process_handler(&self) -> Option<BrowserProcessHandler> {
            Some(HostBrowserProcessHandler::new(
                self.result_pipe.clone(),
                self.cancel_pipe.clone(),
                self.sign_in_url.clone(),
                self.header_token.clone(),
            ))
        }
    }
}

wrap_browser_process_handler! {
    struct HostBrowserProcessHandler {
        result_pipe: String,
        cancel_pipe: Option<String>,
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
            let mut task = OpenSignInWindow::new(
                self.result_pipe.clone(),
                self.cancel_pipe.clone(),
                self.sign_in_url.clone(),
                self.header_token.clone(),
            );
            post_task(ThreadId::UI, Some(&mut task));
        }
    }
}

wrap_task! {
    struct OpenSignInWindow {
        result_pipe: String,
        cancel_pipe: Option<String>,
        sign_in_url: String,
        header_token: String,
    }

    impl Task {
        fn execute(&self) {
            open_sign_in_window(
                self.result_pipe.clone(),
                self.cancel_pipe.clone(),
                self.sign_in_url.clone(),
                self.header_token.clone(),
            );
        }
    }
}

wrap_browser_view_delegate! {
    struct AlloyBrowserView {}

    impl ViewDelegate {}

    impl BrowserViewDelegate {
        // See `SignInWindowDelegate::window_runtime_style` (window.rs) for why
        // this must be Alloy rather than the Chrome-style default.
        fn browser_runtime_style(&self) -> RuntimeStyle {
            RuntimeStyle::ALLOY
        }
    }
}

fn open_sign_in_window(
    result_pipe: String,
    cancel_pipe: Option<String>,
    sign_in_url: String,
    header_token: String,
) {
    let result_pipe = match connect_result_pipe(&result_pipe) {
        Ok(pipe) => pipe,
        Err(e) => {
            log::error!("could not connect the result pipe: {e}");
            quit_message_loop();
            return;
        }
    };
    // Best-effort: no way left to report a result at all if this fails, but
    // the sign-in itself does not depend on cancellation working.
    let cancel_pipe = cancel_pipe.and_then(|name| match connect_cancel_pipe(&name) {
        Ok(pipe) => Some(pipe),
        Err(e) => {
            log::error!("could not connect the cancel pipe: {e}");
            None
        }
    });
    let inner = SignInHandler::new(header_token, result_pipe, cancel_pipe);
    let mut client = SignInClient::new(inner);

    let browser_settings = BrowserSettings::default();
    log::info!("creating the browser view");
    // Created with no URL, as the C++ did. Passing the sign-in URL here starts
    // the navigation from inside `add_child_view`, which is where the browser
    // itself gets created; the window delegate loads it once that has finished.
    let mut browser_view_delegate = AlloyBrowserView::new();
    let browser_view = browser_view_create(
        Some(&mut client),
        None,
        Some(&browser_settings),
        None,
        None,
        Some(&mut browser_view_delegate),
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
