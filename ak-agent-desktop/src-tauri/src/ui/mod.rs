use tauri::{Manager, WebviewUrl, WebviewWindowBuilder};

pub const WINDOW_LABEL: &str = "main";
#[cfg(debug_assertions)]
pub const WINDOW_TITLE: &str = "authentik Agent (Dev)";
#[cfg(not(debug_assertions))]
pub const WINDOW_TITLE: &str = "authentik Agent";

#[cfg(target_os = "macos")]
pub mod macos;

pub fn hide_to_tray(app: &tauri::AppHandle) {
    if let Some(win) = app.get_webview_window(WINDOW_LABEL) {
        let _ = win.hide();
    }
    #[cfg(target_os = "macos")]
    let _ = app.set_activation_policy(tauri::ActivationPolicy::Accessory);
}

pub fn show_main(app: &tauri::AppHandle) {
    #[cfg(target_os = "macos")]
    {
        let _ = app.set_activation_policy(tauri::ActivationPolicy::Regular);
        let _ = app.show();
    }
    let win = match app.get_webview_window(WINDOW_LABEL) {
        Some(w) => w,
        None => {
            let b = WebviewWindowBuilder::new(app, WINDOW_LABEL, WebviewUrl::default())
                .title(WINDOW_TITLE)
                .inner_size(870.0, 640.0);
            #[cfg(target_os = "macos")]
            let b = b
                .hidden_title(true)
                .title_bar_style(tauri::TitleBarStyle::Overlay)
                .traffic_light_position(tauri::LogicalPosition::new(16, 28));
            b.build().unwrap()
        }
    };
    let _ = win.show();
    let _ = win.set_focus();
}
