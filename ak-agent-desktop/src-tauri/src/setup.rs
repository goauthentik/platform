use ak_agent::{Agent, token::global::ProfilesReconciled};
use eyre::{ContextCompat, Result};
use tauri::{
    App, Emitter, Manager,
    tray::{MouseButton, TrayIconBuilder, TrayIconEvent},
};
use tauri_plugin_deep_link::DeepLinkExt;

use crate::{deep_link, ui};

pub fn setup_deeplink(app: &mut App) -> Result<()> {
    // The scheme is registered by the installers, not at runtime.
    for url in app.deep_link().get_current()?.unwrap_or_default() {
        deep_link::handle(app.handle(), &url)?;
    }
    let deep_link_handle = app.handle().clone();
    app.deep_link().on_open_url(move |e| {
        for url in e.urls() {
            deep_link::handle(&deep_link_handle, &url).unwrap_or_else(|e| {
                tracing::warn!("failed to handle URL: {e:?}");
            });
        }
    });
    Ok(())
}

pub fn setup_tray(app: &mut App) -> Result<()> {
    let tray = TrayIconBuilder::new()
        .icon(
            app.default_window_icon()
                .context("failed to get window icon")?
                .clone(),
        )
        .icon_as_template(true)
        .on_tray_icon_event(|tray, e| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                ..
            } = e
            {
                ui::show_main(tray.app_handle()).unwrap_or_else(|e| {
                    tracing::warn!("failed to open window: {e:?}");
                });
            }
        });
    // Add a tray icon menu on linux, otherwise the tray icon doesn't show up at all
    // Also on windows we show the quit item as otherwise there's no way to quit
    #[cfg(any(target_os = "linux", target_os = "windows"))]
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
                ui::show_main(app).unwrap_or_else(|e| {
                    tracing::warn!("failed to open window: {e:?}");
                });
            } else if event.id() == "tray_quit" {
                app.exit(0);
            }
        })
    };
    tray.build(app)?;
    Ok(())
}

pub fn setup_agent(app: &mut App, agent: Agent) {
    let handle = app.handle().clone();
    // Reconciled, not config reload: the UI must not refresh before new profiles' managers exist.
    agent.bus.on(move |_: ProfilesReconciled| {
        let handle = handle.clone();
        async move {
            let visible = handle
                .get_webview_window(ui::WINDOW_LABEL)
                .and_then(|w| w.is_visible().ok())
                .unwrap_or(false);
            if visible && let Err(e) = handle.emit("ak-config-reloaded", ()) {
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
