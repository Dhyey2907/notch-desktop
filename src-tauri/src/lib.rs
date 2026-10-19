mod ask;
mod bridge;
mod calendar;
mod claude;
mod clipboard;
mod commands;
mod control;
mod day;
mod db;
mod desk;
mod developer;
mod error;
mod events;
mod expander;
mod files;
mod focus;
mod fun;
mod game;
mod git;
mod github;
mod grab;
mod habits;
mod history;
mod logging;
mod mail;
mod media;
mod meeting;
mod music;
mod oauth;
mod oauth_clients;
mod phone;
mod powerplan;
mod prices;
mod quick;
mod readlater;
mod secrets;
mod settings;
mod shelf;
mod shell;
mod spotify;
mod state;
mod system;
mod todo;
mod vault;
mod wallpaper;

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
            commands::shell::shell_preview_offset,
            commands::shell::blur_picture,
            commands::shell::blur_toggle,
            commands::shell::blur_off,
            commands::shell::away_show,
            commands::shell::lockscreen_update,
            commands::shell::lockscreen_off,
            commands::shell::shell_is_fullscreen,
            commands::shell::shell_set_fullscreen_hidden,
            commands::shell::shell_list_screens,
            commands::settings::settings_get,
            commands::settings::settings_update,
            commands::settings::settings_defaults,
            commands::settings::shortcut_problems,
            commands::settings::settings_export,
            commands::settings::settings_import,
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
            commands::calendar::calendar_create_event,
            commands::calendar::calendar_quick_add,
            commands::calendar::calendar_respond,
            commands::readlater::read_later_snapshot,
            commands::readlater::read_later_add,
            commands::readlater::read_later_set_read,
            commands::readlater::read_later_remove,
            commands::readlater::read_later_clear_read,
            commands::readlater::read_later_summarize,
            commands::quick::capture_sort,
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
            commands::signin::github_notifications,
            commands::signin::github_mark_read,
            commands::spotify::spotify_disconnect,
            commands::spotify::spotify_refresh_playlists,
            commands::spotify::spotify_play_playlist,
            commands::spotify::spotify_now,
            commands::spotify::spotify_like,
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
            commands::claude::agent_preview,
            commands::claude::claude_test_approval,
            commands::todo::todo_remind_at,
            commands::claude::codex_usage,
            commands::claude::agent_connect,
            commands::claude::agent_disconnect,
            commands::claude::claude_approvals_pending,
            commands::claude::claude_approval_decide,
            commands::claude::claude_approval_audit,
            commands::claude::claude_status_line_preview,
            commands::claude::claude_status_line_enable,
            commands::claude::claude_status_line_disable,
            commands::clipboard::clipboard_snapshot,
            commands::clipboard::clipboard_copy,
            commands::clipboard::clipboard_text,
            commands::clipboard::clipboard_pin,
            commands::clipboard::clipboard_unpin,
            commands::clipboard::clipboard_delete,
            commands::clipboard::clipboard_clear,
            commands::ask::ask_snapshot,
            commands::ask::ask_question,
            commands::ask::ask_stop,
            commands::ask::ask_explain_screen,
            commands::ask::ask_pc,
            commands::ask::research_start,
            commands::prices::price_list,
            commands::prices::price_add,
            commands::prices::price_check,
            commands::prices::price_set_target,
            commands::prices::price_remove,
            commands::desk::habit_dismiss,
            commands::desk::sticky_list,
            commands::desk::sticky_pin,
            commands::desk::sticky_place,
            commands::desk::sticky_color,
            commands::desk::sticky_unpin,
            commands::history::history_add,
            commands::history::history_list,
            commands::history::history_clear,
            commands::meeting::meeting_state,
            commands::meeting::meeting_install,
            commands::meeting::meeting_start,
            commands::meeting::meeting_stop,
            commands::meeting::meeting_cancel,
            commands::ask::rewrite_session,
            commands::ask::rewrite_run,
            commands::ask::rewrite_apply,
            commands::ask::rewrite_cancel,
            commands::ask::ask_clear,
            commands::ask::ai_key_saved,
            commands::ask::ai_set_key,
            commands::ask::ai_clear_key,
            commands::ask::ai_test,
            commands::ask::ai_ollama_models,
            commands::music::lyrics_current,
            commands::music::artwork_palette,
            commands::system::sound_state,
            commands::system::sound_set_volume,
            commands::system::sound_toggle_mute,
            commands::system::bluetooth_devices,
            commands::system::system_dashboard,
            commands::system::system_stats,
            commands::system::brightness_set,
            commands::system::desktop_switch,
            commands::system::weather_refresh,
            commands::system::open_location_settings,
            commands::bridge::bridge_state,
            commands::bridge::bridge_start,
            commands::bridge::bridge_stop,
            commands::shelf::shelf_snapshot,
            commands::shelf::shelf_add,
            commands::shelf::shelf_remove,
            commands::shelf::shelf_clear,
            commands::shelf::shelf_copy,
            commands::shelf::shelf_reveal,
            commands::shelf::shelf_drag,
            commands::shelf::shelf_pin,
            commands::shelf::screenshot_text,
            commands::shelf::screenshot_annotate,
            commands::clipboard::clipboard_stack,
            commands::clipboard::clipboard_copy_private,
            commands::claude::claude_session_usage,
            commands::claude::claude_focus_session,
            commands::claude::claude_projects,
            commands::claude::claude_launch,
            commands::system::system_lock,
            commands::system::system_power,
            commands::system::apps_list,
            commands::system::app_launch,
            commands::system::app_close,
            commands::system::system_monitor,
            commands::system::disk_health,
            commands::system::audio_mixer,
            commands::system::audio_set_app_volume,
            commands::system::audio_set_app_mute,
            commands::system::audio_set_output,
            commands::wallpaper::wallpaper_accent,
            commands::wallpaper::wallpaper_state,
            commands::wallpaper::wallpaper_add,
            commands::wallpaper::wallpaper_pick,
            commands::wallpaper::wallpaper_remove,
            commands::wallpaper::wallpaper_pick_folder,
            commands::wallpaper::wallpaper_set_folder,
            commands::wallpaper::wallpaper_set_target,
            commands::wallpaper::wallpaper_clear_recent,
            commands::wallpaper::wallpaper_apply,
            commands::game::game_status,
            commands::game::game_mark,
            commands::game::game_setup_fps,
            commands::control::control_snapshot,
            commands::control::control_speed,
            commands::control::control_data_usage,
            commands::control::wifi_networks,
            commands::control::wifi_scan,
            commands::control::wifi_connect,
            commands::control::wifi_open_settings,
            commands::control::display_toggles,
            commands::control::set_night_light,
            commands::control::set_hdr,
            commands::control::set_refresh_rate,
            commands::control::open_touchpad_settings,
            commands::control::control_net_apps,
            commands::control::control_speed_test_last,
            commands::control::control_speed_test,
            commands::control::control_refresh,
            commands::control::control_set_radio,
            commands::control::control_open_settings,
            commands::files::files_downloads,
            commands::files::files_recent,
            commands::files::files_convert_image,
            commands::files::files_shrink_image,
            commands::files::files_merge_pdfs,
            commands::files::files_open,
            commands::files::files_reveal,
            commands::files::files_drag,
            commands::files::recorder_state,
            commands::files::recorder_start,
            commands::files::screenrec_start,
            commands::files::files_recent_downloads,
            commands::files::screenrec_stop,
            commands::files::screenrec_state,
            commands::files::recorder_stop,
            commands::vault::vault_state,
            commands::vault::vault_choose_folder,
            commands::vault::vault_save,
            commands::vault::vault_delete,
            commands::vault::vault_copy,
            commands::vault::vault_reveal,
            commands::vault::vault_generate_password,
            commands::vault::vault_export_key,
            commands::vault::vault_import_key,
            commands::vault::totp_codes,
            commands::vault::totp_add,
            commands::vault::totp_delete,
            commands::vault::totp_copy,
            commands::vault::snippets_list,
            commands::vault::snippet_save,
            commands::vault::snippet_delete,
            commands::vault::grab_begin,
            commands::vault::grab_picture,
            commands::vault::grab_finish,
            commands::vault::grab_cancel,
            commands::vault::qr_svg,
            commands::developer::dev_servers,
            commands::developer::dev_stop_server,
            commands::developer::coding_stats,
            commands::developer::dev_run,
            commands::developer::dev_widgets,
            commands::developer::git_suggest_message,
            commands::fun::rgb_music_color,
            commands::system::screen_time_day,
            commands::system::screen_time_days,
            commands::fun::rgb_status,
            commands::fun::pet_buddy_show,
            commands::fun::pet_buddy_hide,
            commands::fun::pet_walking,
            commands::fun::pet_capture,
            commands::fun::pet_media_place,
            commands::fun::pet_coding_in_front,
            commands::desk::desk_windows,
            commands::desk::desk_set_on_top,
            commands::desk::desk_set_opacity,
            commands::desk::desk_focus,
            commands::desk::desk_mini_show,
            commands::desk::desk_mini_hide,
            commands::desk::desk_snap,
            commands::desk::workspace_list,
            commands::desk::workspace_save,
            commands::desk::workspace_restore,
            commands::desk::workspace_delete,
            commands::desk::workspace_resume_offer,
            commands::desk::desk_grid_show,
            commands::desk::desk_grid_hide,
            commands::desk::pip_open,
            commands::desk::pip_source,
            commands::desk::pip_place,
            commands::desk::pip_close,
            commands::desk::routine_run,
            commands::desk::routine_pick_app,
            commands::desk::routine_pick_folder,
            commands::day::mail_state,
            commands::day::mail_refresh,
            commands::day::mail_archive,
            commands::day::mail_mark_read,
            commands::day::mail_snooze,
            commands::day::day_state,
            commands::day::day_plan,
            commands::day::day_refresh,
            commands::focus::focus_state,
            commands::focus::focus_start,
            commands::focus::focus_stop,
            commands::system::system_idle_ms,
            commands::todo::todo_snapshot,
            commands::todo::todo_add_task,
            commands::todo::todo_add_reminder,
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

/// Dev builds only: `NOTCH_DEMO=1` runs a separate copy for recording the website's videos —
/// its own data folder and sign-in store, sample data everywhere, and nothing that reads the
/// user's PC (clipboard, phone, location, network, AI transcripts). Always false in release.
pub(crate) fn demo() -> bool {
    cfg!(debug_assertions) && std::env::var("NOTCH_DEMO").is_ok_and(|v| v == "1")
}

/// Dev builds only: `NOTCH_MOCK=media,calendar` swaps those adapters for mocks (plan.md
/// Rule 9); demo mode mocks them all. Always false in release builds.
fn mock_enabled(module: &str) -> bool {
    cfg!(debug_assertions)
        && (demo()
            || std::env::var("NOTCH_MOCK")
                .map(|v| v.split(',').any(|m| m.trim().eq_ignore_ascii_case(module)))
                .unwrap_or(false))
}

fn media_port(app: &tauri::AppHandle) -> Result<Box<dyn media::MediaPort>, Box<dyn std::error::Error>> {
    #[cfg(debug_assertions)]
    if mock_enabled("media") && !demo() {
        tracing::info!("using mock media adapter");
        return Ok(Box::new(media::mock::MockMediaPort::default()));
    }
    let artwork_dir = app.path().app_cache_dir()?.join("artwork");
    Ok(Box::new(media::windows::WindowsMediaPort::new(artwork_dir)))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Started by Codex or Gemini CLI as a hook: relay the event and exit, no window.
    if let Some(code) = claude::relay::from_args() {
        std::process::exit(code);
    }
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
            commands::settings::migrate_autostart(handle);
            let demo = demo();
            let paths = AppPaths {
                data_dir: if demo {
                    handle.path().app_data_dir()?.join("demo")
                } else {
                    handle.path().app_data_dir()?
                },
                log_dir: handle.path().app_log_dir()?,
            };
            std::fs::create_dir_all(&paths.data_dir)?;
            std::fs::create_dir_all(&paths.log_dir)?;

            app.manage(logging::init(&paths.log_dir));
            let version = handle.package_info().version.to_string();
            tracing::info!(%version, "starting Overline");

            let paths_db = paths.data_dir.join("notch.db");
            let wallpapers_dir = paths.data_dir.join("wallpapers");
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
            if !demo {
                github.start();
            }
            app.manage(github);

            let phone = phone::service::PhoneService::new(handle.clone());
            if !demo {
                phone.start();
            }
            app.manage(phone);

            let clips_dir = app.path().app_cache_dir()?.join("clips");
            let clipboard = clipboard::service::ClipboardService::new(handle.clone(), clips_dir);
            if !demo {
                clipboard.start();
            }
            app.manage(clipboard);

            let shelf = shelf::service::ShelfService::new(handle.clone());
            shelf.start();
            app.manage(shelf);
            app.manage(wallpaper::service::WallpaperService::new(
                handle.clone(),
                wallpapers_dir,
                app.path().app_cache_dir()?.join("wallpapers").join("thumbs"),
            ));
            let readlater = readlater::service::ReadLaterService::new(handle.clone());
            if demo {
                readlater.seed_demo();
            }
            app.manage(readlater);
            let vault = vault::service::VaultService::new(handle.clone());
            if !demo {
                vault.start();
            }
            app.manage(vault);
            let expander = expander::service::ExpanderService::new(handle.clone());
            if !demo {
                expander.start();
            }
            app.manage(expander);
            app.manage(grab::service::GrabService::new(handle.clone()));
            let developer = developer::service::DeveloperService::new(handle.clone());
            if !demo {
                developer.start();
            }
            app.manage(developer);
            let files = files::service::FilesService::new(handle.clone());
            if !demo {
                files.start();
            }
            app.manage(files);
            app.manage(bridge::service::BridgeService::new(handle.clone()));

            let system = system::service::SystemService::new(handle.clone());
            if !demo {
                system.start();
            }
            app.manage(system);

            // After the system service: it reads Bluetooth batteries from there.
            let control = control::service::ControlService::new(handle.clone());
            app.manage(control.clone());
            if !demo {
                control.start();
            }

            let music = music::service::MusicService::new(
                handle.clone(),
                app.path().app_cache_dir()?.join("artwork"),
            );
            music.start();
            app.manage(music);

            app.manage(ask::service::AskService::new(
                handle.clone(),
                app.path().app_cache_dir()?.join("ask"),
            ));

            let game = game::service::GameService::new(
                handle.clone(),
                app.path().app_local_data_dir()?.join("tools"),
            );
            if !demo {
                game.start();
            }
            app.manage(game);

            let focus = focus::service::FocusService::new(handle.clone());
            focus.run();
            app.manage(focus);

            let todo = todo::service::TodoService::new(handle.clone());
            todo.start();
            app.manage(todo);

            // After Calendar and To-do: they use its sign-in and tasks.
            let mail = mail::service::MailService::new(handle.clone());
            app.manage(mail.clone());
            mail.start();
            app.manage(meeting::service::MeetingService::new(handle.clone()));
            let prices = prices::service::PriceService::new(handle.clone());
            app.manage(prices.clone());
            if !demo {
                prices.start();
            }
            let desk = desk::service::DeskService::new(handle.clone());
            app.manage(desk.clone());
            if !demo {
                desk.start();
                desk.start_session_keeper();
                let stickies = app
                    .state::<state::AppState>()
                    .settings
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .desk
                    .stickies
                    .clone();
                for s in &stickies {
                    if let Err(error) = desk::sticky::show(handle, s) {
                        tracing::warn!(kind = error.kind(), "could not reopen a sticky note");
                    }
                }
            }
            let fun = fun::service::FunService::new(handle.clone());
            app.manage(fun.clone());
            fun.start();
            let day = day::service::DayService::new(handle.clone());
            app.manage(day.clone());
            day.start();

            let spotify = spotify::service::SpotifyService::new(handle.clone(), mock_enabled("spotify"));
            spotify.start();
            app.manage(spotify);

            let git = git::service::GitService::new(handle.clone(), mock_enabled("git"));
            git.start();
            app.manage(git);

            let claude = claude::service::ClaudeService::new(handle.clone(), paths_db.clone())?;
            if !demo {
                claude.start();
            }
            app.manage(claude);
            shell::init(handle)?;
            Ok(())
        })
        .invoke_handler(app_commands!())
        .run(tauri::generate_context!())
        .expect("error while running Overline");
}
