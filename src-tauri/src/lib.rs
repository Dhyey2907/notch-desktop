mod commands;
mod db;
mod error;
mod events;
mod logging;
mod state;

use tauri::Manager;

use crate::{
    db::Db,
    events::{AppEvent, EventBus},
    state::{AppPaths, AppState},
};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        // Must be registered first: a second launch focuses the running instance instead.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
            }
        }))
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let handle = app.handle();
            let paths = AppPaths {
                data_dir: handle.path().app_data_dir()?,
                log_dir: handle.path().app_log_dir()?,
            };
            std::fs::create_dir_all(&paths.data_dir)?;
            std::fs::create_dir_all(&paths.log_dir)?;

            app.manage(logging::init(&paths.log_dir));
            let version = handle.package_info().version.to_string();
            tracing::info!(%version, "starting Notch Desktop");

            let db = Db::open(&paths.data_dir.join("notch.db"))?;
            let bus = EventBus::new(EventBus::tauri_emitter(handle.clone()));
            bus.emit("app", AppEvent::AppReady { version });

            app.manage(AppState { db, bus, paths });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::app::app_info,
            commands::app::diagnostics_ping,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Notch Desktop");
}
