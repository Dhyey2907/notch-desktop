use tauri::{AppHandle, State};
use tauri_plugin_autostart::ManagerExt;

use crate::{
    error::{AppError, AppResult},
    events::AppEvent,
    settings::{self, Settings},
    shell,
    state::AppState,
};

pub(crate) fn persist_and_broadcast(state: &AppState, next: Settings) -> AppResult<Settings> {
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

/// Default values, for "Reset this page".
#[tauri::command]
pub fn settings_defaults() -> Settings {
    Settings::default()
}

/// Shortcuts the user picked that couldn't be registered (bad combination or taken).
#[tauri::command]
pub fn shortcut_problems() -> Vec<shell::ShortcutProblem> {
    shell::shortcut_problems()
}

/// Save settings to a JSON file the user picks. Returns where, or `None` if cancelled.
#[tauri::command]
pub async fn settings_export(app: AppHandle, state: State<'_, AppState>) -> AppResult<Option<String>> {
    use tauri_plugin_dialog::DialogExt;
    let mut settings = state.settings.lock().unwrap_or_else(|e| e.into_inner()).clone();
    // Accounts and per-PC state don't travel.
    settings.calendar.account_email = None;
    settings.spotify.account_name = None;
    settings.focus.sound_owner.clear();
    settings.focus.sleep_until_ms = None;
    let json = serde_json::to_string_pretty(&settings).map_err(|e| AppError::Platform(e.to_string()))?;
    let Some(path) = app
        .dialog()
        .file()
        .set_file_name("notch-settings.json")
        .add_filter("Settings", &["json"])
        .blocking_save_file()
    else {
        return Ok(None);
    };
    let path = path.into_path().map_err(|e| AppError::Platform(e.to_string()))?;
    std::fs::write(&path, json)?;
    Ok(Some(path.display().to_string()))
}

/// Load settings from a JSON file the user picks. Accounts already connected stay connected.
#[tauri::command]
pub async fn settings_import(app: AppHandle, state: State<'_, AppState>) -> AppResult<Option<Settings>> {
    use tauri_plugin_dialog::DialogExt;
    let Some(path) = app
        .dialog()
        .file()
        .add_filter("Settings", &["json"])
        .blocking_pick_file()
    else {
        return Ok(None);
    };
    let path = path.into_path().map_err(|e| AppError::Platform(e.to_string()))?;
    let text = std::fs::read_to_string(path)?;
    if text.len() > 1_000_000 {
        return Err(AppError::InvalidInput(
            "That file is too big to be Overline settings".into(),
        ));
    }
    let mut next: Settings = serde_json::from_str(&text)
        .map_err(|_| AppError::InvalidInput("That file isn't Overline settings".into()))?;
    let current = state.settings.lock().unwrap_or_else(|e| e.into_inner()).clone();
    next.calendar.account_email = current.calendar.account_email;
    next.spotify.account_name = current.spotify.account_name;
    next.first_run_completed = current.first_run_completed;
    persist_and_broadcast(&state, next).map(Some)
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
pub async fn open_settings_window(app: AppHandle, section: Option<String>) -> AppResult<()> {
    shell::open_settings_window(&app, section.as_deref())
}

/// The app used to be called "Notch Desktop", and "Start with Windows" is stored under the app's
/// name. Move an old entry to the new name once, so the setting survives the rename.
pub fn migrate_autostart(app: &AppHandle) {
    #[cfg(windows)]
    {
        use windows::{
            core::w,
            Win32::System::Registry::{RegDeleteKeyValueW, RegGetValueW, HKEY_CURRENT_USER, RRF_RT_REG_SZ},
        };
        let key = w!(r"Software\Microsoft\Windows\CurrentVersion\Run");
        let old = w!("Notch Desktop");
        // SAFETY: only asks whether the value exists (no buffer), then deletes that one value.
        let had_old =
            unsafe { RegGetValueW(HKEY_CURRENT_USER, key, old, RRF_RT_REG_SZ, None, None, None) }.is_ok();
        if !had_old {
            return;
        }
        let _ = unsafe { RegDeleteKeyValueW(HKEY_CURRENT_USER, key, old) };
        match app.autolaunch().enable() {
            Ok(()) => tracing::info!("moved start-with-Windows to the new app name"),
            Err(error) => tracing::warn!(%error, "could not re-register start-with-Windows"),
        }
    }
    #[cfg(not(windows))]
    let _ = app;
}
