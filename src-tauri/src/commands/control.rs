use tauri::{AppHandle, State};

use crate::{
    control::{service::ControlService, ControlSnapshot, NetSpeed, RadioKind},
    error::{AppError, AppResult},
};

#[tauri::command]
pub fn control_snapshot(control: State<'_, ControlService>) -> ControlSnapshot {
    control.snapshot()
}

/// Read radios, battery and accessories again now (the panel was opened).
#[tauri::command]
pub async fn control_refresh(control: State<'_, ControlService>) -> AppResult<ControlSnapshot> {
    let control = control.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        control.refresh();
        control.snapshot()
    })
    .await
    .map_err(|e| AppError::Platform(e.to_string()))
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

/// Apps using the internet now, with an estimated rate each (poll while the panel is open).
#[tauri::command]
pub async fn control_net_apps(
    control: State<'_, ControlService>,
) -> AppResult<Vec<crate::control::netapps::AppTraffic>> {
    Ok(control.inner().clone().net_apps().await)
}

#[tauri::command]
pub fn control_speed_test_last(
    control: State<'_, ControlService>,
) -> Option<crate::control::speedtest::SpeedTest> {
    control.speed_test()
}

/// Runs a speed test against speed.cloudflare.com (about 20 seconds).
#[tauri::command]
pub async fn control_speed_test(
    control: State<'_, ControlService>,
) -> AppResult<crate::control::speedtest::SpeedTest> {
    control.inner().clone().run_speed_test().await
}

/// Data used today and this month.
#[tauri::command]
pub fn control_data_usage(control: State<'_, ControlService>) -> AppResult<crate::control::usage::DataUsage> {
    control.data_usage()
}

fn blocking_err(e: tauri::Error) -> crate::error::AppError {
    crate::error::AppError::Platform(e.to_string())
}

/// Wi-Fi networks nearby (one per name; connected, then known, then strongest).
#[tauri::command]
pub async fn wifi_networks() -> AppResult<Vec<crate::control::wifi::WifiNetwork>> {
    #[cfg(windows)]
    return tauri::async_runtime::spawn_blocking(crate::control::wifi::win::networks)
        .await
        .map_err(blocking_err)?;
    #[cfg(not(windows))]
    Ok(Vec::new())
}

/// Ask the Wi-Fi adapter to look again.
#[tauri::command]
pub async fn wifi_scan() -> AppResult<()> {
    #[cfg(windows)]
    return tauri::async_runtime::spawn_blocking(crate::control::wifi::win::scan)
        .await
        .map_err(blocking_err)?;
    #[cfg(not(windows))]
    Ok(())
}

/// Join a network this PC already knows (by name).
#[tauri::command]
pub async fn wifi_connect(ssid: String) -> AppResult<()> {
    #[cfg(windows)]
    return tauri::async_runtime::spawn_blocking(move || {
        let list = crate::control::wifi::win::networks()?;
        let n = list
            .into_iter()
            .find(|n| n.ssid == ssid && n.known)
            .ok_or_else(|| {
                crate::error::AppError::InvalidInput("Join that network in Windows first".into())
            })?;
        crate::control::wifi::win::connect(&n.profile)
    })
    .await
    .map_err(blocking_err)?;
    #[cfg(not(windows))]
    {
        let _ = ssid;
        Ok(())
    }
}

/// Windows' own Wi-Fi list (to join a new network with its password).
#[tauri::command]
pub fn wifi_open_settings(app: tauri::AppHandle) -> AppResult<()> {
    use tauri_plugin_opener::OpenerExt;
    app.opener()
        .open_url("ms-availablenetworks:", None::<&str>)
        .map_err(|e| crate::error::AppError::Platform(e.to_string()))
}

/// Night light, HDR and refresh rate now.
#[tauri::command]
pub async fn display_toggles() -> AppResult<crate::control::toggles::DisplayToggles> {
    #[cfg(windows)]
    return tauri::async_runtime::spawn_blocking(crate::control::toggles::win::read)
        .await
        .map_err(blocking_err);
    #[cfg(not(windows))]
    Ok(Default::default())
}

/// Turn night light on or off; when Windows' setting can't be flipped here, its page opens.
#[tauri::command]
pub fn set_night_light(on: bool, app: tauri::AppHandle) -> AppResult<bool> {
    #[cfg(windows)]
    if crate::control::toggles::win::set_night_light(on).is_ok() {
        return Ok(true);
    }
    let _ = on;
    use tauri_plugin_opener::OpenerExt;
    app.opener()
        .open_url("ms-settings:nightlight", None::<&str>)
        .map_err(|e| crate::error::AppError::Platform(e.to_string()))?;
    Ok(false)
}

#[tauri::command]
pub fn set_hdr(on: bool) -> AppResult<()> {
    #[cfg(windows)]
    return crate::control::toggles::win::set_hdr(on);
    #[cfg(not(windows))]
    {
        let _ = on;
        Ok(())
    }
}

#[tauri::command]
pub fn set_refresh_rate(hz: u32) -> AppResult<()> {
    #[cfg(windows)]
    return crate::control::toggles::win::set_rate(hz);
    #[cfg(not(windows))]
    {
        let _ = hz;
        Ok(())
    }
}

/// Touchpads have no public on/off switch: open Windows' touchpad page.
#[tauri::command]
pub fn open_touchpad_settings(app: tauri::AppHandle) -> AppResult<()> {
    use tauri_plugin_opener::OpenerExt;
    app.opener()
        .open_url("ms-settings:devices-touchpad", None::<&str>)
        .map_err(|e| crate::error::AppError::Platform(e.to_string()))
}
