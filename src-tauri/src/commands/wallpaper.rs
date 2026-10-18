use tauri::{AppHandle, State};

use crate::{
    error::{AppError, AppResult},
    wallpaper::{service::WallpaperService, WallpaperState},
};

/// Run file and COM work off the main thread.
async fn blocking<T: Send + 'static>(
    wallpaper: &WallpaperService,
    work: impl FnOnce(WallpaperService) -> AppResult<T> + Send + 'static,
) -> AppResult<T> {
    let wallpaper = wallpaper.clone();
    tauri::async_runtime::spawn_blocking(move || work(wallpaper))
        .await
        .map_err(|e| AppError::Platform(e.to_string()))?
}

#[tauri::command]
pub async fn wallpaper_state(wallpaper: State<'_, WallpaperService>) -> AppResult<WallpaperState> {
    blocking(&wallpaper, |w| Ok(w.snapshot())).await
}

/// Save copies of these pictures; returns how many were new.
#[tauri::command]
pub async fn wallpaper_add(paths: Vec<String>, wallpaper: State<'_, WallpaperService>) -> AppResult<usize> {
    blocking(&wallpaper, move |w| w.add(&paths)).await
}

/// Choose pictures to save; returns how many were new.
#[tauri::command]
pub async fn wallpaper_pick(app: AppHandle, wallpaper: State<'_, WallpaperService>) -> AppResult<usize> {
    use tauri_plugin_dialog::DialogExt;
    let picked = tauri::async_runtime::spawn_blocking(move || {
        app.dialog()
            .file()
            .set_title("Choose wallpapers")
            .add_filter("Pictures", &["jpg", "jpeg", "png", "webp", "bmp"])
            .blocking_pick_files()
    })
    .await
    .map_err(|e| AppError::Platform(e.to_string()))?;
    let paths: Vec<String> = picked
        .unwrap_or_default()
        .into_iter()
        .filter_map(|p| p.into_path().ok())
        .map(|p| p.to_string_lossy().into_owned())
        .collect();
    if paths.is_empty() {
        return Ok(0);
    }
    blocking(&wallpaper, move |w| w.add(&paths)).await
}

#[tauri::command]
pub async fn wallpaper_remove(path: String, wallpaper: State<'_, WallpaperService>) -> AppResult<()> {
    blocking(&wallpaper, move |w| w.remove(&path)).await
}

/// Choose the folder shown under "My folder"; false if cancelled.
#[tauri::command]
pub async fn wallpaper_pick_folder(
    app: AppHandle,
    wallpaper: State<'_, WallpaperService>,
) -> AppResult<bool> {
    use tauri_plugin_dialog::DialogExt;
    let picked = tauri::async_runtime::spawn_blocking(move || {
        app.dialog()
            .file()
            .set_title("Choose your wallpaper folder")
            .blocking_pick_folder()
    })
    .await
    .map_err(|e| AppError::Platform(e.to_string()))?;
    let Some(folder) = picked
        .and_then(|p| p.into_path().ok())
        .map(|p| p.to_string_lossy().into_owned())
    else {
        return Ok(false);
    };
    blocking(&wallpaper, move |w| w.set_folder(Some(folder))).await?;
    Ok(true)
}

/// Set (or with none, forget) the folder shown under "My folder".
#[tauri::command]
pub async fn wallpaper_set_folder(
    folder: Option<String>,
    wallpaper: State<'_, WallpaperService>,
) -> AppResult<()> {
    blocking(&wallpaper, move |w| w.set_folder(folder)).await
}

/// Which screen a click changes: `all` or a screen id.
#[tauri::command]
pub async fn wallpaper_set_target(target: String, wallpaper: State<'_, WallpaperService>) -> AppResult<()> {
    blocking(&wallpaper, move |w| w.set_target(target)).await
}

#[tauri::command]
pub async fn wallpaper_clear_recent(wallpaper: State<'_, WallpaperService>) -> AppResult<()> {
    blocking(&wallpaper, |w| w.clear_recent()).await
}

/// Set a picture as the desktop wallpaper on `target` (`all` or a screen id).
#[tauri::command]
pub async fn wallpaper_apply(
    path: String,
    target: String,
    wallpaper: State<'_, WallpaperService>,
) -> AppResult<()> {
    blocking(&wallpaper, move |w| w.apply(&path, &target)).await
}

/// Two colours from the main screen's wallpaper, for "Accent from wallpaper"; none if it can't
/// be read.
#[tauri::command]
pub async fn wallpaper_accent(
    wallpaper: State<'_, WallpaperService>,
) -> AppResult<Option<crate::music::Palette>> {
    blocking(&wallpaper, |w| Ok(w.accent())).await
}
