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

/// Lock the PC (the Windows lock screen).
#[tauri::command]
pub fn system_lock() -> AppResult<()> {
    #[cfg(windows)]
    {
        // SAFETY: no arguments; Windows shows the lock screen.
        unsafe { windows::Win32::System::Shutdown::LockWorkStation() }
            .map_err(|e| AppError::Platform(format!("could not lock: {e}")))
    }
    #[cfg(not(windows))]
    Err(AppError::Platform("not supported".into()))
}

/// What the power menu can do.
#[derive(Debug, Clone, Copy, serde::Deserialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum PowerAction {
    Sleep,
    Hibernate,
    Restart,
    /// Restart into the firmware (BIOS/UEFI) setup; Windows asks for admin first.
    RestartBios,
    Shutdown,
    SignOut,
}

/// Sleep, hibernate, restart, shut down or sign out. The UI confirms first.
#[tauri::command]
pub fn system_power(action: PowerAction) -> AppResult<()> {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        let run = |args: &[&str]| -> AppResult<()> {
            std::process::Command::new("shutdown.exe")
                .args(args)
                .creation_flags(CREATE_NO_WINDOW)
                .spawn()
                .map(|_| ())
                .map_err(|e| AppError::Platform(format!("could not run shutdown: {e}")))
        };
        match action {
            PowerAction::Sleep | PowerAction::Hibernate => {
                let hibernate = matches!(action, PowerAction::Hibernate);
                // SAFETY: plain flags; Windows suspends the machine and returns on wake.
                let ok = unsafe { windows::Win32::System::Power::SetSuspendState(hibernate, false, false) };
                if ok {
                    Ok(())
                } else if hibernate {
                    Err(AppError::Platform("Hibernate is turned off on this PC".into()))
                } else {
                    Err(AppError::Platform("could not sleep".into()))
                }
            }
            PowerAction::Restart => run(&["/r", "/t", "0"]),
            PowerAction::Shutdown => run(&["/s", "/t", "0"]),
            PowerAction::SignOut => run(&["/l"]),
            // `/fw` needs admin: go through UAC.
            PowerAction::RestartBios => std::process::Command::new("powershell.exe")
                .args([
                    "-NoProfile",
                    "-Command",
                    "Start-Process shutdown.exe -ArgumentList '/r','/fw','/t','0' -Verb RunAs",
                ])
                .creation_flags(CREATE_NO_WINDOW)
                .spawn()
                .map(|_| ())
                .map_err(|e| AppError::Platform(format!("could not restart to BIOS: {e}"))),
        }
    }
    #[cfg(not(windows))]
    {
        let _ = action;
        Err(AppError::Platform("not supported".into()))
    }
}

/// Each app's volume, and the output devices.
#[tauri::command]
pub async fn audio_mixer() -> AppResult<crate::system::mixer::Mixer> {
    tauri::async_runtime::spawn_blocking(crate::system::mixer::read)
        .await
        .map_err(|e| AppError::Platform(e.to_string()))?
}

#[tauri::command]
pub async fn audio_set_app_volume(pid: u32, level: f32) -> AppResult<()> {
    tauri::async_runtime::spawn_blocking(move || crate::system::mixer::set_app_volume(pid, level))
        .await
        .map_err(|e| AppError::Platform(e.to_string()))?
}

#[tauri::command]
pub async fn audio_set_app_mute(pid: u32, muted: bool) -> AppResult<()> {
    tauri::async_runtime::spawn_blocking(move || crate::system::mixer::set_app_mute(pid, muted))
        .await
        .map_err(|e| AppError::Platform(e.to_string()))?
}

/// Send sound to another output device.
#[tauri::command]
pub async fn audio_set_output(id: String) -> AppResult<()> {
    tauri::async_runtime::spawn_blocking(move || crate::system::mixer::set_output(&id))
        .await
        .map_err(|e| AppError::Platform(e.to_string()))?
}

/// Every app in the Start menu (for the palette's launcher).
#[tauri::command]
pub async fn apps_list() -> AppResult<Vec<crate::system::apps::InstalledApp>> {
    tauri::async_runtime::spawn_blocking(crate::system::apps::list)
        .await
        .map_err(|e| AppError::Platform(e.to_string()))?
}

#[tauri::command]
pub fn app_launch(id: String) -> AppResult<()> {
    crate::system::apps::launch(&id)
}

/// Ask every window of an app to close (like clicking its X); returns how many were asked.
#[tauri::command]
pub fn app_close(exe: String) -> AppResult<usize> {
    #[cfg(windows)]
    {
        let mut key = crate::clipboard::exe_key(&exe);
        if !key.ends_with(".exe") {
            key.push_str(".exe");
        }
        Ok(crate::desk::win::close_app(&key))
    }
    #[cfg(not(windows))]
    {
        let _ = exe;
        Err(AppError::Platform("not supported".into()))
    }
}

/// CPU, memory and GPU load with the apps using the most (the system monitor asks each few seconds).
#[tauri::command]
pub async fn system_monitor() -> AppResult<crate::system::procs::SystemMonitor> {
    #[cfg(windows)]
    {
        tauri::async_runtime::spawn_blocking(|| crate::system::procs::read(5))
            .await
            .map_err(|e| AppError::Platform(e.to_string()))
    }
    #[cfg(not(windows))]
    Err(AppError::Platform("not supported".into()))
}

/// Each drive's health and how full each volume is.
#[tauri::command]
pub async fn disk_health(fresh: bool) -> AppResult<crate::system::disks::DiskHealth> {
    tauri::async_runtime::spawn_blocking(move || crate::system::disks::read(fresh))
        .await
        .map_err(|e| AppError::Platform(e.to_string()))?
}
