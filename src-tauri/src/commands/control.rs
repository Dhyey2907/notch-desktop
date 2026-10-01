use tauri::{AppHandle, State};

use crate::{
    control::{service::ControlService, ControlSnapshot, NetSpeed, RadioKind},
    error::{AppError, AppResult},
};

#[tauri::command]
pub fn control_snapshot(control: State<'_, ControlService>) -> ControlSnapshot {
    control.snapshot()
}

/// Download and upload speed since the last call; the open panel asks every second.
#[tauri::command]
pub fn control_speed(control: State<'_, ControlService>) -> NetSpeed {
    control.speed()
}

/// Turn Wi-Fi, Bluetooth or the mobile hotspot on or off.
#[tauri::command]
pub async fn control_set_radio(
    kind: RadioKind,
    on: bool,
    control: State<'_, ControlService>,
) -> AppResult<()> {
    let control = control.inner().clone();
    tauri::async_runtime::spawn_blocking(move || control.set_radio(kind, on))
        .await
        .map_err(|e| AppError::Platform(e.to_string()))?
}

/// Open a Windows Settings page for things apps can't switch themselves.
#[tauri::command]
pub fn control_open_settings(page: String, app: AppHandle) -> AppResult<()> {
    use tauri_plugin_opener::OpenerExt;
    let uri = match page.as_str() {
        "nightlight" => "ms-settings:nightlight",
        "airplane" => "ms-settings:network-airplanemode",
        "wifi" => "ms-settings:network-wifi",
        "bluetooth" => "ms-settings:bluetooth",
        "hotspot" => "ms-settings:network-mobilehotspot",
        "battery" => "ms-settings:batterysaver",
        "radios" => "ms-settings:privacy-radios",
        _ => return Err(AppError::InvalidInput("unknown settings page".into())),
    };
    app.opener()
        .open_url(uri, None::<&str>)
        .map_err(|e| AppError::Platform(e.to_string()))
}
