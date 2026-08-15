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

        // `add_child_view` is where CEF attaches the browser to the widget and
        // brings up its compositor, and that is the frame that dies on the
        // secure desktop while the same call succeeds under `CPUS_CREDUI` in
        // CI. The secure desktop has no usable accelerated path — Chromium's
        // own log shows DirectComposition degrading there — and this window is
        // a small login form that software rendering serves fine. Applied in
        // every scenario so what CI exercises is what ships.
        fn on_before_command_line_processing(
            &self,
            _process_type: Option<&CefString>,
            command_line: Option<&mut CommandLine>,
        ) {
            let Some(command_line) = command_line else {
                return;
            };
            for switch in [
                "disable-gpu",
                "disable-gpu-compositing",
                "disable-direct-composition",
            ] {
                command_line.append_switch(Some(&CefString::from(switch)));
            }
        }
    }
}

wrap_browser_process_handler! {
    struct HostBrowserProcessHandler {
        result_pipe: usize,
        cancel_pipe: Option<usize>,
    }

    impl BrowserProcessHandler {
        fn on_context_initialized(&self) {
            let start = match crate::sysd::sys_auth_start_async() {
                Ok(s) => s,
                Err(e) => {
                    log::error!("sys_auth_start_async failed: {e}");
                    let mut pipe = file_from_raw_handle(self.result_pipe);
                    let _ = wire::write_auth_result(
                        &mut pipe,
                        &wire::AuthResult::Failed { reason: e.to_string() },
                    );
                    quit_message_loop();
                    return;
                }
            };

            let result_pipe = file_from_raw_handle(self.result_pipe);
            let cancel_pipe = self.cancel_pipe.map(file_from_raw_handle);
            let inner = SignInHandler::new(start.header_token, result_pipe, cancel_pipe);
            let mut client = SignInClient::new(inner);

            let browser_settings = BrowserSettings::default();
            let url = CefString::from(start.url.as_str());
            log::info!("creating the browser view");
            let browser_view = browser_view_create(
                Some(&mut client),
                Some(&url),
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
            let mut delegate = SignInWindowDelegate::new(std::cell::RefCell::new(browser_view));
            window_create_top_level(Some(&mut delegate));
            log::info!("sign-in window requested");
        }
    }
}
