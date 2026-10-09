use ak_meta::full_version;
use ak_platform::{log::LogBuilder, string::PlatformString};
use eyre::Result;
use sentry::ClientInitGuard;

mod cmd;
mod deep_link;
mod setup;
mod ui;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let mut opts = ak_meta::sentry_options();
    opts.auto_session_tracking = true;
    let guard = ak_meta::sentry_init("ak-agent-desktop", opts);
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
    #[allow(unused_mut)]
    let mut context = tauri::generate_context!();
    #[cfg(debug_assertions)]
    {
        context.config_mut().identifier = "io.goauthentik.platform.dev.agent.desktop".to_string();
    }

    // Own runtime instead of tauri's default so its threads get a recognisable name.
    let rt = tokio::runtime::Builder::new_multi_thread()
        .thread_name("ak-agent-desktop")
        .enable_all()
        .build()?;
    tauri::async_runtime::set(rt.handle().clone());

    // Created before tauri so commands never see unmanaged state.
    let agent = rt.block_on(ak_agent::agent::Agent::new())?;

    tauri::Builder::default()
        .manage(agent.clone())
        .plugin(tauri_plugin_sentry::init(&guard))
        .plugin(tauri_plugin_os::init())
        .plugin(tauri_plugin_single_instance::init(|app, argv, _cwd| {
            // Deep links are forwarded to `on_open_url` and routed there.
            if !argv.iter().any(|a| a.starts_with(deep_link::SCHEME)) {
                ui::show_main(app);
            }
        }))
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_deep_link::init())
        .setup(move |app| {
            setup::setup_deeplink(app)?;
            setup::setup_agent(app, agent);

            #[cfg(target_os = "macos")]
            ui::macos::setup_app(app)?;
            setup::setup_tray(app)?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            cmd::profile::get_user_info,
            cmd::profile::list_profiles,
            cmd::profile::active_profile,
            cmd::profile::setup_profile,
            cmd::profile::delete_profile,
            cmd::ssh::get_ssh_config,
            cmd::ssh::set_ssh_fallback_agent,
            cmd::ssh::get_ssh_status,
            cmd::version::get_versions,
        ])
        .build(context)?
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
