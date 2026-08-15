//! The sign-in window's chrome: fixed size, centered, framed but not
//! resizable/minimizable/maximizable, matching the current CEF window.

use cef::*;

wrap_window_delegate! {
    pub struct SignInWindowDelegate {
        browser_view: std::cell::RefCell<Option<BrowserView>>,
        url: String,
    }

    impl ViewDelegate {
        fn preferred_size(&self, _view: Option<&mut View>) -> Size {
            Size {
                width: wire::WINDOW_WIDTH,
                height: wire::WINDOW_HEIGHT,
            }
        }
    }

    impl PanelDelegate {}

    impl WindowDelegate {
        fn on_window_created(&self, window: Option<&mut Window>) {
            let browser_view = self.browser_view.borrow();
            let (Some(window), Some(browser_view)) = (window, browser_view.as_ref()) else {
                log::error!("sign-in window created without a window or a browser view");
                return;
            };
            let mut view = View::from(browser_view);
            window.add_child_view(Some(&mut view));
            window.center_window(Some(&Size {
                width: wire::WINDOW_WIDTH,
                height: wire::WINDOW_HEIGHT,
            }));

            if let Some(mut icon) = crate::icon::load() {
                window.set_window_icon(Some(&mut icon));
                window.set_window_app_icon(Some(&mut icon));
            }

            // `show()` alone only makes the window visible; on Windows it
            // does not reliably take the foreground, especially once
            // credprovider's spawn and CEF's own multi-process startup have
            // put enough time between process launch and this callback for
            // the OS to stop treating it as belonging to that launch.
            // `activate()` is CEF's dedicated bring-to-front-and-focus call,
            // paired with `credprovider` granting this process foreground
            // rights via `AllowSetForegroundWindow` right after spawning it.
            window.show();
            window.activate();
            log::info!("sign-in window shown");

            // The browser only exists once the view is parented, so this is the
            // earliest the sign-in URL can be loaded.
            match browser_view.browser().and_then(|b| b.main_frame()) {
                Some(frame) => {
                    frame.load_url(Some(&CefString::from(self.url.as_str())));
                    log::info!("loading the sign-in URL");
                }
                None => log::error!("no main frame to load the sign-in URL into"),
            }
        }

        fn on_window_destroyed(&self, _window: Option<&mut Window>) {
            log::info!("sign-in window destroyed");
            *self.browser_view.borrow_mut() = None;
        }

        fn can_close(&self, _window: Option<&mut Window>) -> i32 {
            let browser_view = self.browser_view.borrow();
            match browser_view.as_ref().and_then(BrowserView::browser) {
                Some(browser) => browser
                    .host()
                    .map(|host| host.try_close_browser())
                    .unwrap_or(1),
                None => 1,
            }
        }

        fn can_resize(&self, _window: Option<&mut Window>) -> i32 {
            0
        }

        fn can_minimize(&self, _window: Option<&mut Window>) -> i32 {
            0
        }

        fn can_maximize(&self, _window: Option<&mut Window>) -> i32 {
            0
        }

        fn is_frameless(&self, _window: Option<&mut Window>) -> i32 {
            0
        }
    }
}
