//! The sign-in window's chrome: fixed size, centered, framed but not
//! resizable/minimizable/maximizable, matching the current CEF window.

use cef::*;

use crate::foreground;

wrap_window_delegate! {
    pub struct SignInWindowDelegate {
        browser_view: std::cell::RefCell<Option<BrowserView>>,
        url: String,
        // Whether the window has ever held the foreground. Shared through an
        // `Rc` because `wrap_window_delegate!` derives a `Clone` that clones
        // every field, so a bare `Cell` would fork the moment CEF took a
        // second reference to the delegate.
        ever_activated: std::rc::Rc<std::cell::Cell<bool>>,
    }

    impl ViewDelegate {
        fn preferred_size(&self, _view: Option<&mut View>) -> Size {
            Size {
                width: ak_ee_wcp_wire::WINDOW_WIDTH,
                height: ak_ee_wcp_wire::WINDOW_HEIGHT,
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
                width: ak_ee_wcp_wire::WINDOW_WIDTH,
                height: ak_ee_wcp_wire::WINDOW_HEIGHT,
            }));

            if let Some(mut icon) = crate::icon::load() {
                window.set_window_icon(Some(&mut icon));
                window.set_window_app_icon(Some(&mut icon));
            }

            // Topmost before the first paint, so the window is never behind
            // LogonUI even for a frame. `activate()` asks for focus, which is
            // a separate and much less certain thing — see `foreground`.
            window.set_always_on_top(1);
            window.show();
            window.activate();
            // So focus, whenever it arrives, lands in the web content rather
            // than on the frame.
            view.request_focus();
            log::info!("sign-in window shown, foreground_pid={:?}", foreground::foreground_pid());
            foreground::schedule_reactivation(window, 0, &self.ever_activated);

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

        /// Separates "never took focus" from "took it and lost it again":
        /// same symptom, different fixes, and `activate()` reports neither.
        fn on_window_activation_changed(&self, _window: Option<&mut Window>, active: ::std::os::raw::c_int) {
            if active != 0 {
                self.ever_activated.set(true);
            }
            log::info!(
                "sign-in window activation changed: active={active} foreground_pid={:?}",
                foreground::foreground_pid()
            );
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

        /// Chrome style pulls in the full Chrome UI/browser layer —
        /// extensions, profile manager, tabs — none of which this
        /// single-purpose sign-in window needs. Alloy is CEF's lighter
        /// embedding-oriented runtime. `open_sign_in_window` sets the same
        /// style on the `BrowserView` — a Chrome style Window can only host
        /// one Chrome style `BrowserView`, so both had to move together.
        fn window_runtime_style(&self) -> RuntimeStyle {
            RuntimeStyle::ALLOY
        }
    }
}
