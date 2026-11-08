use ak_agent::Agent;
use eyre::Result;
use tauri::{
    App, Emitter, Manager,
    tray::{MouseButton, TrayIconBuilder, TrayIconEvent},
};
use tauri_plugin_deep_link::DeepLinkExt;

use crate::{deep_link, ui};

pub fn setup_deeplink(app: &mut App) -> Result<()> {
    // The scheme is registered by the installers, not at runtime.
    for url in app.deep_link().get_current()?.unwrap_or_default() {
        deep_link::handle(app.handle(), &url);
    }
    let deep_link_handle = app.handle().clone();
    app.deep_link().on_open_url(move |e| {
        for url in e.urls() {
            deep_link::handle(&deep_link_handle, &url);
        }
    });
    Ok(())
}

pub fn setup_tray(app: &mut App) -> Result<()> {
    let tray = TrayIconBuilder::new()
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
        });
    // Add a tray icon menu on linux, otherwise the tray icon doesn't show up at all
    #[cfg(target_os = "linux")]
    let tray = {
        use tauri::menu::{MenuBuilder, MenuItemBuilder};

        let open_item = MenuItemBuilder::with_id("tray_open", format!("Open {}", ui::WINDOW_TITLE))
            .build(app)?;
        let quit_item = MenuItemBuilder::with_id("tray_quit", "Quit").build(app)?;
        let menu = MenuBuilder::new(app)
            .item(&open_item)
            .separator()
            .item(&quit_item)
            .build()?;
        tray.menu(&menu).on_menu_event(|app, event| {
            if event.id() == "tray_open" {
                ui::show_main(app);
            } else if event.id() == "tray_quit" {
                app.exit(0);
            }
        })
    };
    tray.build(app)?;
    Ok(())
}

pub fn setup_agent(app: &mut App, agent: Agent) {
    let watcher_handle = app.handle().clone();
    let reload_notify = agent.cfg.on_reload();
    tauri::async_runtime::spawn(async move {
        loop {
            reload_notify.notified().await;
            let visible = watcher_handle
                .get_webview_window(ui::WINDOW_LABEL)
                .and_then(|w| w.is_visible().ok())
                .unwrap_or(false);
            if visible && let Err(e) = watcher_handle.emit("ak-config-reloaded", ()) {
                tracing::warn!("failed to emit config reload event: {e}");
            }
        }
    });
    tauri::async_runtime::spawn(async move {
        if let Err(e) = agent.start().await {
            tracing::error!("agent exited with error: {e}");
        }
    });
}
