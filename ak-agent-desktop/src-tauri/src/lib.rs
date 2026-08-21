use ak_meta::full_version;
use ak_platform::{log::LogBuilder, string::PlatformString};
use eyre::Result;
use sentry::ClientInitGuard;
use tauri::tray::{MouseButton, TrayIconBuilder, TrayIconEvent};
use tauri::{Emitter, Manager};

mod cmd;
mod ui;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let mut opts = ak_meta::sentry_options("ak-agent-desktop");
    opts.auto_session_tracking = true;
    let guard = sentry::init(opts);
    LogBuilder::new(
        PlatformString::new()
            .with_windows("authentik User Service")
            .with_linux("ak-agent"),
    )
    .with_default_filters()
    .enable();
    tracing::trace!("authentik Agent Desktop v{}", full_version());

    match start_tauri(guard) {
        Ok(_) => {}
        Err(e) => {
            tracing::error!("Failed to start tauri: {e:?}");
        }
    }
}

pub fn start_tauri(guard: ClientInitGuard) -> Result<()> {
    tauri::Builder::default()
        .plugin(tauri_plugin_sentry::init(&guard))
        .plugin(tauri_plugin_os::init())
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            ui::show_main(app);
        }))
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                match ak_agent::agent::Agent::new().await {
                    Ok(agent) => {
                        handle.manage(agent.clone());
                        let watcher_handle = handle.clone();
                        let reload_notify = agent.cfg.on_reload();
                        tauri::async_runtime::spawn(async move {
                            loop {
                                reload_notify.notified().await;
                                let visible = watcher_handle
                                    .get_webview_window(ui::WINDOW_LABEL)
                                    .and_then(|w| w.is_visible().ok())
                                    .unwrap_or(false);
                                if visible
                                    && let Err(e) = watcher_handle.emit("ak-config-reloaded", ())
                                {
                                    tracing::warn!("failed to emit config reload event: {e}");
                                }
                            }
                        });
                        if let Err(e) = agent.start().await {
                            tracing::error!("agent exited with error: {e}");
                        }
                    }
                    Err(e) => tracing::error!("failed to start agent: {e}"),
                }
            });

            #[cfg(target_os = "macos")]
            ui::macos::setup_app(app)?;

            // Registers (or confirms registration of) the privileged CTRL
            // relay daemon. Best-effort: a failure here (e.g. not yet
            // approved in System Settings) shouldn't block the rest of the
            // app — it just means elevated CTRL actions aren't available yet.
            #[cfg(target_os = "macos")]
            if let Err(e) = ak_platform::net::elevate::ensure_registered() {
                tracing::warn!("failed to register sysd CTRL relay daemon: {e:?}");
            }

            TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .icon_as_template(true)
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
        .invoke_handler(tauri::generate_handler![
            cmd::get_user_info,
            cmd::list_profiles,
            cmd::active_profile,
            cmd::get_versions,
        ])
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
