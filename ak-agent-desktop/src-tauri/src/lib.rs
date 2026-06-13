use ak_platform::prelude::*;
use ak_platform::{keyring, log::init_log, string::PlatformString};
use tauri::tray::{MouseButton, TrayIconBuilder, TrayIconEvent};

mod cmd;
mod ui;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    init_log(
        PlatformString::new()
            .with_windows("authentik User Service")
            .with_linux("ak-agent"),
    );
    match keyring::init() {
        Ok(_) => {}
        Err(e) => {
            eprintln!("Failed to setup keyring: {e:?}");
        }
    };

    match start_tauri() {
        Ok(_) => {}
        Err(e) => {
            log::error!("Failed to start tauri: {e:?}");
        }
    }
}

pub fn start_tauri() -> Result<()> {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            ui::show_main(app);
        }))
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            tauri::async_runtime::spawn(async move {
                match ak_agent::agent::Agent::new().await {
                    Ok(agent) => {
                        if let Err(e) = agent.start().await {
                            log::error!("agent exited with error: {e}");
                        }
                    }
                    Err(e) => log::error!("failed to start agent: {e}"),
                }
            });

            #[cfg(target_os = "macos")]
            ui::macos::setup_menu(app)?;

            TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .on_tray_icon_event(|tray, e| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        ..
                    } = e
                    {
                        ui::show_main(tray.app_handle());
                    }
                })
                .build(app)?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![cmd::greet,])
        .build(tauri::generate_context!())?
        .run(|app, event| {
            if let tauri::RunEvent::ExitRequested { code, api, .. } = event
                && code.is_none()
            {
                api.prevent_exit();
                ui::hide_to_tray(app);
            }
        });
    Ok(())
}
