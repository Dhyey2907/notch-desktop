use tauri::State;

use crate::{
    error::{AppError, AppResult},
    system::{service::SystemService, BluetoothDevice, SoundState},
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
