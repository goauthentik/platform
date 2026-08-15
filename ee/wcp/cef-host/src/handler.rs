//! Browser-level callbacks: header injection on every request, redirect
//! interception to complete the sign-in flow, and quitting the message loop
//! once the window closes.

use std::fs::File;
use std::io::Read;
use std::os::windows::io::FromRawHandle;
use std::sync::{Arc, Mutex, Weak};

use cef::*;
use wire::AuthResult;

pub struct SignInHandler {
    header_token: String,
    result_pipe: Mutex<Option<File>>,
    browser_list: Vec<Browser>,
    weak_self: Weak<Mutex<Self>>,
}

impl SignInHandler {
    pub fn new(header_token: String, result_pipe: File, cancel_pipe: Option<File>) -> Arc<Mutex<Self>> {
        let handler = Arc::new_cyclic(|weak| {
            Mutex::new(Self {
                header_token,
                result_pipe: Mutex::new(Some(result_pipe)),
                browser_list: Vec::new(),
                weak_self: weak.clone(),
            })
        });

        if let Some(cancel_pipe) = cancel_pipe {
            watch_cancel_pipe(cancel_pipe, handler.clone());
        }

        handler
    }

    fn on_after_created(&mut self, browser: Option<&mut Browser>) {
        if let Some(browser) = browser.cloned() {
            self.browser_list.push(browser);
        }
    }

    fn on_before_close(&mut self, browser: Option<&mut Browser>) {
        let mut browser = browser.cloned();
        if let Some(pos) = browser
            .as_mut()
            .and_then(|b| self.browser_list.iter().position(|e| e.is_same(Some(b)) != 0))
        {
            self.browser_list.remove(pos);
        }
        if self.browser_list.is_empty() {
            signal_if_unsent(&self.result_pipe, AuthResult::Cancelled);
            quit_message_loop();
        }
    }

    fn close_all_browsers(&mut self) {
        for browser in &self.browser_list {
            if let Some(host) = browser.host() {
                host.close_browser(1);
            }
        }
    }

    /// Sends the sign-in outcome once, then closes the window. Safe to call
    /// more than once — only the first call's result is sent.
    fn complete(&self, result: AuthResult) {
        signal_if_unsent(&self.result_pipe, result);
        let Some(this) = self.weak_self.upgrade() else {
            return;
        };
        if currently_on(ThreadId::UI) != 0 {
            close_all_browsers(&this);
        } else {
            let mut task = CloseAllBrowsers::new(this);
            post_task(ThreadId::UI, Some(&mut task));
        }
    }
}

fn close_all_browsers(handler: &Arc<Mutex<SignInHandler>>) {
    let mut inner = handler.lock().unwrap_or_else(|e| e.into_inner());
    inner.close_all_browsers();
}

fn signal_if_unsent(result_pipe: &Mutex<Option<File>>, result: AuthResult) {
    let mut guard = result_pipe.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(mut file) = guard.take()
        && let Err(e) = wire::write_auth_result(&mut file, &result)
    {
        log::error!("failed to write result to pipe: {e}");
    }
}

wrap_task! {
    struct CloseAllBrowsers {
        inner: Arc<Mutex<SignInHandler>>,
    }

    impl Task {
        fn execute(&self) {
            close_all_browsers(&self.inner);
        }
    }
}

/// Blocks on the control pipe in a background thread; on any signal (or the
/// pipe closing), asks the UI thread to close the sign-in window.
fn watch_cancel_pipe(mut pipe: File, handler: Arc<Mutex<SignInHandler>>) {
    std::thread::spawn(move || {
        let mut buf = [0u8; 1];
        let _ = pipe.read(&mut buf);
        let inner = handler.lock().unwrap_or_else(|e| e.into_inner());
        inner.complete(AuthResult::Cancelled);
    });
}

wrap_client! {
    pub struct SignInClient {
        inner: Arc<Mutex<SignInHandler>>,
    }

    impl Client {
        fn life_span_handler(&self) -> Option<LifeSpanHandler> {
            Some(SignInLifeSpanHandler::new(self.inner.clone()))
        }

        fn request_handler(&self) -> Option<RequestHandler> {
            Some(SignInRequestHandler::new(self.inner.clone()))
        }
    }
}

wrap_life_span_handler! {
    struct SignInLifeSpanHandler {
        inner: Arc<Mutex<SignInHandler>>,
    }

    impl LifeSpanHandler {
        fn on_after_created(&self, browser: Option<&mut Browser>) {
            let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
            inner.on_after_created(browser);
        }

        fn on_before_close(&self, browser: Option<&mut Browser>) {
            let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
            inner.on_before_close(browser);
        }
    }
}

wrap_request_handler! {
    struct SignInRequestHandler {
        inner: Arc<Mutex<SignInHandler>>,
    }

    impl RequestHandler {
        fn resource_request_handler(
            &self,
            _browser: Option<&mut Browser>,
            _frame: Option<&mut Frame>,
            _request: Option<&mut Request>,
            _is_navigation: i32,
            _is_download: i32,
            _request_initiator: Option<&CefString>,
            _disable_default_handling: Option<&mut i32>,
        ) -> Option<ResourceRequestHandler> {
            Some(SignInResourceRequestHandler::new(self.inner.clone()))
        }
    }
}

wrap_resource_request_handler! {
    struct SignInResourceRequestHandler {
        inner: Arc<Mutex<SignInHandler>>,
    }

    impl ResourceRequestHandler {
        fn on_before_resource_load(
            &self,
            _browser: Option<&mut Browser>,
            _frame: Option<&mut Frame>,
            request: Option<&mut Request>,
            _callback: Option<&mut Callback>,
        ) -> ReturnValue {
            let Some(request) = request else {
                return ReturnValue::CONTINUE;
            };

            let inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
            request.set_header_by_name(
                Some(&CefString::from(wire::AUTH_HEADER_NAME)),
                Some(&CefString::from(inner.header_token.as_str())),
                1,
            );

            let url = CefString::from(&request.url()).to_string();
            if !url.starts_with(wire::REDIRECT_PREFIX) {
                return ReturnValue::CONTINUE;
            }

            let result = match sysd_client::sys_auth_url(&url) {
                Ok(Some(token)) => AuthResult::Completed {
                    username: token.username,
                },
                Ok(None) => AuthResult::Failed {
                    reason: "token validation failed".to_string(),
                },
                Err(e) => AuthResult::Failed {
                    reason: e.to_string(),
                },
            };
            inner.complete(result);
            ReturnValue::CANCEL
        }
    }
}

/// Wraps an inherited raw pipe handle as an owned `File`. Called once at
/// startup for each of the two pipes this process was handed.
pub fn file_from_raw_handle(handle: usize) -> File {
    unsafe { File::from_raw_handle(handle as *mut std::ffi::c_void) }
}
