use tauri::{
    App,
    menu::{MenuBuilder, MenuItemBuilder, SubmenuBuilder},
};

pub fn setup_app(app: &mut App) -> std::result::Result<(), Box<dyn std::error::Error>> {
    app.set_activation_policy(tauri::ActivationPolicy::Accessory);
    setup_menu(app)
}

pub fn setup_menu(app: &mut App) -> std::result::Result<(), Box<dyn std::error::Error>> {
    let hide_item = MenuItemBuilder::with_id("hide_to_tray", "Hide to Tray")
        .accelerator("Cmd+Q")
        .build(app)?;
    let quit_item = MenuItemBuilder::with_id("quit", "Quit")
        .accelerator("Cmd+Shift+Q")
        .build(app)?;

    let app_name = app.config().product_name.as_deref().unwrap_or("App");
    let app_submenu = SubmenuBuilder::new(app, app_name)
        .about(None)
        .separator()
        .services()
        .separator()
        .hide()
        .hide_others()
        .show_all()
        .separator()
        .item(&hide_item)
        .item(&quit_item)
        .build()?;

    let edit_submenu = SubmenuBuilder::new(app, "Edit")
        .undo()
        .redo()
        .separator()
        .cut()
        .copy()
        .paste()
        .select_all()
        .build()?;

    let window_submenu = SubmenuBuilder::new(app, "Window")
        .minimize()
        .separator()
        .close_window()
        .build()?;

    app.set_menu(
        MenuBuilder::new(app)
            .item(&app_submenu)
            .item(&edit_submenu)
            .item(&window_submenu)
            .build()?,
    )?;

    app.on_menu_event(|app, event| {
        if event.id() == "hide_to_tray" {
            super::hide_to_tray(app);
        } else if event.id() == "quit" {
            app.exit(0);
        }
    });
    Ok(())
}
