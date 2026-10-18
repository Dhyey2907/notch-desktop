use tauri::{State, WebviewWindow};

use crate::{
    error::{AppError, AppResult},
    shell::{self, ShellState},
};

/// Largest logical dimension the island may request; guards against runaway sizes.
const MAX_LOGICAL_DIMENSION: f64 = 2000.0;

fn validate_dimension(name: &str, value: f64) -> AppResult<()> {
    if value.is_finite() && value > 0.0 && value <= MAX_LOGICAL_DIMENSION {
        Ok(())
    } else {
        Err(AppError::InvalidInput(format!("{name} out of range: {value}")))
    }
}

/// Resize the island window (logical pixels) and keep it pinned top-center.
#[tauri::command]
pub fn shell_set_window_size(
    window: WebviewWindow,
    shell: State<'_, ShellState>,
    width: f64,
    height: f64,
) -> AppResult<()> {
    validate_dimension("width", width)?;
    validate_dimension("height", height)?;
    shell.set_window_size(window.label(), (width, height));
    shell::place(&window, (width, height))
}

/// Whether a fullscreen app is in front on this island's screen (the island hides).
#[tauri::command]
pub fn shell_is_fullscreen(window: WebviewWindow) -> bool {
    shell::is_fullscreen(window.label())
}

/// Hide the island during a fullscreen app (`true`), or bring it back (`false`).
#[tauri::command]
pub fn shell_set_fullscreen_hidden(window: WebviewWindow, hidden: bool) -> AppResult<()> {
    shell::set_fullscreen_hidden(&window, hidden)
}

/// Connected screens, for "Show the notch on" in Settings.
#[tauri::command]
pub fn shell_list_screens(app: tauri::AppHandle) -> Vec<shell::islands::ScreenInfo> {
    shell::islands::list_screens(&app)
}

/// Alt+drag: show the notch at this sideways offset while dragging (`null` ends the preview).
#[tauri::command]
pub fn shell_preview_offset(window: WebviewWindow, offset_x: Option<i32>) -> AppResult<()> {
    shell::preview_offset(&window, offset_x)
}

/// The blurred picture for this privacy cover window.
#[tauri::command]
pub fn blur_picture(window: WebviewWindow) -> Option<String> {
    shell::blur::picture(window.label())
}

/// Cover every screen with a blur, or take the cover away.
#[tauri::command]
pub async fn blur_toggle(app: tauri::AppHandle) -> AppResult<()> {
    tauri::async_runtime::spawn_blocking(move || shell::blur::toggle(&app))
        .await
        .map_err(|e| crate::error::AppError::Platform(e.to_string()))?
}

/// Take the privacy cover away.
#[tauri::command]
pub fn blur_off(app: tauri::AppHandle) {
    shell::blur::off(&app);
}

/// Draw these lines onto the wallpaper and set it as the lock-screen picture (rate-limited;
/// returns whether it was updated).
#[tauri::command]
pub async fn lockscreen_update(lines: Vec<String>, app: tauri::AppHandle) -> AppResult<bool> {
    use tauri::Manager;
    if lines.len() > 20 {
        return Err(crate::error::AppError::InvalidInput("too many lines".into()));
    }
    let on = app
        .state::<crate::state::AppState>()
        .settings
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .appearance
        .lockscreen_info;
    if !on {
        return Ok(false);
    }
    let cache = app.path().app_cache_dir()?;
    let monitor = app
        .primary_monitor()?
        .ok_or_else(|| crate::error::AppError::Platform("no screen".into()))?;
    let size = (monitor.size().width, monitor.size().height);
    tauri::async_runtime::spawn_blocking(move || {
        #[cfg(windows)]
        {
            let wallpaper = crate::wallpaper::win::screens()?
                .into_iter()
                .find(|s| s.primary)
                .and_then(|s| s.current)
                .ok_or_else(|| crate::error::AppError::Platform("no wallpaper picture to draw on".into()))?;
            shell::lockscreen::update(&cache, std::path::Path::new(&wallpaper), size, &lines)
        }
        #[cfg(not(windows))]
        {
            let _ = (cache, size, lines);
            Ok(false)
        }
    })
    .await
    .map_err(|e| crate::error::AppError::Platform(e.to_string()))?
}

/// Lock-screen info turned off: put the old picture back. `false`: it was Windows Spotlight
/// (or unknown), so the lock-screen settings were opened instead.
#[tauri::command]
pub async fn lockscreen_off(app: tauri::AppHandle) -> AppResult<bool> {
    use tauri::Manager;
    let cache = app.path().app_cache_dir()?;
    let restored = tauri::async_runtime::spawn_blocking(move || {
        #[cfg(windows)]
        return shell::lockscreen::restore(&cache);
        #[cfg(not(windows))]
        {
            let _ = cache;
            false
        }
    })
    .await
    .map_err(|e| crate::error::AppError::Platform(e.to_string()))?;
    if !restored {
        use tauri_plugin_opener::OpenerExt;
        let _ = app.opener().open_url("ms-settings:lockscreen", None::<&str>);
    }
    Ok(restored)
}
