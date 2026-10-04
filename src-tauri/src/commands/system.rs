use tauri::State;

use crate::{
    error::{AppError, AppResult},
    system::{service::SystemService, BluetoothDevice, Dashboard, SoundState, SystemStats},
};

#[tauri::command]
pub fn sound_state(system: State<'_, SystemService>) -> Option<SoundState> {
    system.sound()
}

/// Set speaker volume (0.0 – 1.0), e.g. from dragging the HUD bar.
#[tauri::command]
pub fn sound_set_volume(level: f32, system: State<'_, SystemService>) -> AppResult<()> {
    if !level.is_finite() || !(0.0..=1.0).contains(&level) {
        return Err(AppError::InvalidInput("volume out of range".into()));
    }
    system.set_volume(level);
    Ok(())
}

#[tauri::command]
pub fn sound_toggle_mute(system: State<'_, SystemService>) {
    system.toggle_mute();
}

/// Bluetooth devices connected now, with battery where they report it.
#[tauri::command]
pub async fn bluetooth_devices(system: State<'_, SystemService>) -> AppResult<Vec<BluetoothDevice>> {
    let system = system.inner().clone();
    tauri::async_runtime::spawn_blocking(move || system.bluetooth())
        .await
        .map_err(|e| AppError::Platform(e.to_string()))
}

/// Everything the Home dashboard shows.
#[tauri::command]
pub fn system_dashboard(system: State<'_, SystemService>) -> AppResult<Dashboard> {
    system.dashboard()
}

/// CPU, memory and battery right now (the view asks while it's open).
#[tauri::command]
pub fn system_stats(system: State<'_, SystemService>) -> SystemStats {
    system.stats()
}

/// Laptop screen brightness, percent.
#[tauri::command]
pub fn brightness_set(level: u8, system: State<'_, SystemService>) -> AppResult<()> {
    if level > 100 {
        return Err(AppError::InvalidInput("brightness out of range".into()));
    }
    system.set_brightness(level)
}

/// Switch to Windows virtual desktop `index` (0-based).
#[tauri::command]
pub async fn desktop_switch(index: u32, system: State<'_, SystemService>) -> AppResult<()> {
    let system = system.inner().clone();
    tauri::async_runtime::spawn_blocking(move || system.switch_desktop(index))
        .await
        .map_err(|e| AppError::Platform(e.to_string()))?
}

#[tauri::command]
pub fn weather_refresh(system: State<'_, SystemService>) {
    system.refresh_weather();
}

/// Open Windows location settings (to allow Notch to read location for weather).
#[tauri::command]
pub fn open_location_settings(app: tauri::AppHandle) -> AppResult<()> {
    use tauri_plugin_opener::OpenerExt;
    app.opener()
        .open_url("ms-settings:privacy-location", None::<&str>)
        .map_err(|e| AppError::Platform(e.to_string()))
}

/// Milliseconds since the last keyboard or mouse input (the pet gets sleepy).
#[tauri::command]
pub fn system_idle_ms() -> u64 {
    #[cfg(windows)]
    return crate::system::screentime::probe::idle_ms();
    #[cfg(not(windows))]
    0
}

/// Screen time on a past day (YYYY-MM-DD), per hour.
#[tauri::command]
pub fn screen_time_day(
    date: String,
    state: tauri::State<'_, crate::state::AppState>,
) -> crate::error::AppResult<crate::system::ScreenTime> {
    let date = chrono::NaiveDate::parse_from_str(&date, "%Y-%m-%d")
        .map_err(|_| crate::error::AppError::InvalidInput("invalid date".into()))?;
    state.db.with(|conn| crate::system::screentime::day(conn, date))
}

/// Minutes per day over the last 30 days (all the history kept), oldest first.
#[tauri::command]
pub fn screen_time_days(
    state: tauri::State<'_, crate::state::AppState>,
) -> crate::error::AppResult<Vec<crate::system::DayMinutes>> {
    let today = chrono::Local::now().date_naive();
    state
        .db
        .with(|conn| crate::system::screentime::recent_days(conn, today, 30))
}
