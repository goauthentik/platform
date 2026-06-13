use tauri::{
    Manager, WebviewUrl, WebviewWindowBuilder,
};

#[cfg(target_os = "macos")]
pub mod macos;

pub fn hide_to_tray(app: &tauri::AppHandle) {
    if let Some(win) = app.get_webview_window("main") {
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

    let win = match app.get_webview_window("main") {
        Some(w) => w,
        None => WebviewWindowBuilder::new(app, "main", WebviewUrl::default())
            .title("App")
            .build()
            .unwrap(),
    };
    let _ = win.show();
    let _ = win.set_focus();
}
