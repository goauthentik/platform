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
        fn on_context_initialized(&self) {
            let start = match sysd_client::sys_auth_start_async() {
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
            let browser_view = browser_view_create(
                Some(&mut client),
                Some(&url),
                Some(&browser_settings),
                None,
                None,
                None,
            );

            let mut delegate = SignInWindowDelegate::new(std::cell::RefCell::new(browser_view));
            window_create_top_level(Some(&mut delegate));
        }
    }
}
