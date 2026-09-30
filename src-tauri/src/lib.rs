mod calendar;
mod claude;
mod commands;
mod db;
mod error;
mod events;
mod git;
mod github;
mod logging;
mod media;
mod oauth;
mod oauth_clients;
mod phone;
mod secrets;
mod settings;
mod shell;
mod spotify;
mod state;
mod todo;

use tauri::Manager;

use crate::{
    db::Db,
    events::{AppEvent, EventBus},
    shell::ShellState,
    state::{AppPaths, AppState},
};

/// Logical size of the collapsed island window; mirrors `windowSizeFor("collapsed")`
/// in `src/shell/geometry.ts` so the first frame is already the right size.
const INITIAL_WINDOW_SIZE: (f64, f64) = (64.0, 20.0);

/// Every command the webview may call. Dev-only commands are compiled into debug builds only.
macro_rules! app_commands {
    () => {{
        #[cfg(debug_assertions)]
        let handler: fn(tauri::ipc::Invoke<tauri::Wry>) -> bool =
            app_commands!(@with commands::dev::dev_emit_test_activity, commands::dev::dev_emit_test_approval);
        #[cfg(not(debug_assertions))]
        let handler: fn(tauri::ipc::Invoke<tauri::Wry>) -> bool = app_commands!(@with);
        handler
    }};
    (@with $($dev:path),*) => {
        tauri::generate_handler![
            commands::app::app_info,
            commands::app::diagnostics_ping,
            commands::app::open_external,
            commands::shell::shell_set_window_size,
            commands::shell::shell_is_fullscreen,
            commands::shell::shell_set_fullscreen_hidden,
            commands::shell::shell_list_screens,
            commands::settings::settings_get,
            commands::settings::settings_update,
            commands::settings::settings_record_module_init,
            commands::settings::autostart_get,
            commands::settings::autostart_set,
            commands::settings::open_settings_window,
            commands::diagnostics::diagnostics_open_log_dir,
            commands::diagnostics::diagnostics_export_logs,
            commands::media::media_get_state,
            commands::media::media_command,
            commands::calendar::calendar_get_snapshot,
            commands::calendar::calendar_connect,
            commands::calendar::calendar_disconnect,
            commands::calendar::calendar_sync_now,
            commands::calendar::calendar_open_link,
            commands::spotify::spotify_get_snapshot,
            commands::spotify::spotify_connect,
            commands::signin::sign_in_options,
            commands::signin::calendar_connect_builtin,
            commands::signin::spotify_connect_builtin,
            commands::signin::github_status,
            commands::signin::github_sign_in,
            commands::signin::github_cancel_sign_in,
            commands::signin::github_sign_out,
            commands::signin::github_repositories,
            commands::signin::github_repo_overview,
            commands::signin::github_select_repo,
            commands::spotify::spotify_disconnect,
            commands::spotify::spotify_refresh_playlists,
            commands::spotify::spotify_play_playlist,
            commands::spotify::spotify_set_pinned,
            commands::git::git_get_snapshot,
            commands::git::git_refresh,
            commands::git::git_pick_repo,
            commands::git::git_select_repo,
            commands::git::git_forget_repo,
            commands::git::git_run,
            commands::git::git_pull_requests,
            commands::git::git_open_folder,
            commands::claude::claude_status,
            commands::claude::claude_usage,
            commands::claude::claude_hooks_preview,
            commands::claude::claude_connect,
            commands::claude::claude_disconnect,
            commands::claude::claude_approvals_pending,
            commands::claude::claude_approval_decide,
            commands::claude::claude_approval_audit,
            commands::claude::claude_status_line_preview,
            commands::claude::claude_status_line_enable,
            commands::claude::claude_status_line_disable,
            commands::todo::todo_snapshot,
            commands::todo::todo_add_task,
            commands::todo::todo_set_done,
            commands::todo::todo_rename,
            commands::todo::todo_delete,
            commands::todo::todo_clear_done,
            commands::todo::todo_start_timer,
            commands::todo::todo_cancel_timer,
            commands::todo::todo_acknowledge,
            commands::todo::note_add,
            commands::todo::note_update,
            commands::todo::note_delete,
            commands::phone::phone_snapshot,
            commands::phone::phone_dismiss,
            commands::phone::phone_open,
            commands::phone::phone_open_access_settings,
            $($dev),*
        ]
    };
}

/// Dev builds only: `NOTCH_MOCK=media,calendar` swaps those adapters for mocks (plan.md
/// Rule 9). Always false in release builds.
fn mock_enabled(module: &str) -> bool {
    cfg!(debug_assertions)
        && std::env::var("NOTCH_MOCK")
            .map(|v| v.split(',').any(|m| m.trim().eq_ignore_ascii_case(module)))
            .unwrap_or(false)
}

fn media_port(app: &tauri::AppHandle) -> Result<Box<dyn media::MediaPort>, Box<dyn std::error::Error>> {
    #[cfg(debug_assertions)]
    if mock_enabled("media") {
        tracing::info!("using mock media adapter");
        return Ok(Box::new(media::mock::MockMediaPort::default()));
    }
    let artwork_dir = app.path().app_cache_dir()?.join("artwork");
    Ok(Box::new(media::windows::WindowsMediaPort::new(artwork_dir)))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        // Must be registered first: a second launch focuses the running instance instead.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(window) = app.get_webview_window(shell::MAIN_WINDOW) {
                let _ = shell::show_island(&window);
            }
        }))
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(
            tauri_plugin_autostart::Builder::new()
                .args([shell::AUTOSTART_ARG])
                .build(),
        )
        .plugin(shell::shortcut_plugin())
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

            let paths_db = paths.data_dir.join("notch.db");
            let db = Db::open(&paths_db)?;
            let settings = db.with(settings::load)?;
            let bus = EventBus::new(EventBus::tauri_emitter(handle.clone()));
            bus.emit("app", AppEvent::AppReady { version });

            app.manage(AppState {
                db,
                bus,
                paths,
                settings: std::sync::Mutex::new(settings),
            });
            app.manage(ShellState::new(INITIAL_WINDOW_SIZE));

            // Integrations connect in the background; a failing adapter never blocks the island.
            let media = media::MediaService::new(media_port(handle)?);
            media.start(media::bus_publisher(handle.clone()));
            app.manage(media);

            let calendar = calendar::service::CalendarService::new(handle.clone(), mock_enabled("calendar"));
            calendar.start();
            app.manage(calendar);

            let github = github::service::GithubService::new(handle.clone());
            github.start();
            app.manage(github);

            let phone = phone::service::PhoneService::new(handle.clone());
            phone.start();
            app.manage(phone);

            let todo = todo::service::TodoService::new(handle.clone());
            todo.start();
            app.manage(todo);

            let spotify = spotify::service::SpotifyService::new(handle.clone(), mock_enabled("spotify"));
            spotify.start();
            app.manage(spotify);

            let git = git::service::GitService::new(handle.clone(), mock_enabled("git"));
            git.start();
            app.manage(git);

            let claude = claude::service::ClaudeService::new(handle.clone(), paths_db.clone())?;
            claude.start();
            app.manage(claude);
            shell::init(handle)?;
            Ok(())
        })
        .invoke_handler(app_commands!())
        .run(tauri::generate_context!())
        .expect("error while running Notch Desktop");
}
