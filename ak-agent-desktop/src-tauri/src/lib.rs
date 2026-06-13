use tauri::{
    Manager, WebviewUrl, WebviewWindowBuilder,
    tray::{MouseButton, TrayIconBuilder, TrayIconEvent},
};
use ak_platform::{keyring, log, string::PlatformString};

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

fn show_main(app: &tauri::AppHandle) {
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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    log::init_log(
        PlatformString::new()
            .with_windows("authentik User Service")
            .with_linux("ak-agent"),
    );
    keyring::init().expect("keyring init failed");
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            show_main(app);
        }))
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .on_tray_icon_event(|tray, e| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        ..
                    } = e
                    {
                        show_main(tray.app_handle());
                    }
                })
                .build(app)?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![greet])
        .build(tauri::generate_context!())
        .expect("error while running tauri application")
        .run(|app, event| {
            if let tauri::RunEvent::ExitRequested { code, api, .. } = event {
                if code.is_none() {
                    api.prevent_exit();
                    if let Some(win) = app.get_webview_window("main") {
                        let _ = win.hide();
                    }
                }
            }
        });
}
