use tauri::{AppHandle, State};
use tauri_plugin_autostart::ManagerExt;

use crate::{
    error::{AppError, AppResult},
    events::AppEvent,
    settings::{self, Settings},
    shell,
    state::AppState,
};

fn persist_and_broadcast(state: &AppState, next: Settings) -> AppResult<Settings> {
    next.validate()?;
    state.db.with(|conn| settings::save(conn, &next))?;
    *state.settings.lock().unwrap_or_else(|e| e.into_inner()) = next.clone();
    // Every window (island and settings) keeps its copy in sync from this event.
    state
        .bus
        .emit("settings", AppEvent::SettingsChanged(next.clone()));
    Ok(next)
}

#[tauri::command]
pub fn settings_get(state: State<'_, AppState>) -> Settings {
    state.settings.lock().unwrap_or_else(|e| e.into_inner()).clone()
}

#[tauri::command]
pub fn settings_update(settings: Settings, state: State<'_, AppState>) -> AppResult<Settings> {
    persist_and_broadcast(&state, settings)
}

/// Report a module's init outcome; the backend owns the failure counter so it survives
/// restarts and can disable a module that keeps crashing on startup.
#[tauri::command]
pub fn settings_record_module_init(
    module_id: String,
    ok: bool,
    state: State<'_, AppState>,
) -> AppResult<Settings> {
    let mut next = state.settings.lock().unwrap_or_else(|e| e.into_inner()).clone();
    let before = next.clone();
    next.record_module_init(&module_id, ok);
    if next == before {
        return Ok(next);
    }
    persist_and_broadcast(&state, next)
}

/// Whether Notch starts at Windows sign-in. The OS registration is the source of truth.
#[tauri::command]
pub fn autostart_get(app: AppHandle) -> AppResult<bool> {
    app.autolaunch()
        .is_enabled()
        .map_err(|e| AppError::Platform(e.to_string()))
}

#[tauri::command]
pub fn autostart_set(enabled: bool, app: AppHandle) -> AppResult<bool> {
    let launcher = app.autolaunch();
    let result = if enabled {
        launcher.enable()
    } else {
        launcher.disable()
    };
    result.map_err(|e| AppError::Platform(e.to_string()))?;
    tracing::info!(enabled, "updated launch-at-login");
    autostart_get(app)
}

/// Must be `async`: creating a webview from a synchronous command deadlocks on Windows
/// (WebView2 initializes on the main thread, which a sync command is blocking).
#[tauri::command]
pub async fn open_settings_window(app: AppHandle) -> AppResult<()> {
    shell::open_settings_window(&app)
}
