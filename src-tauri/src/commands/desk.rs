use tauri::{AppHandle, State, WebviewWindow};

use crate::{
    desk::{service::DeskService, DeskWindow},
    error::{AppError, AppResult},
};

#[tauri::command]
pub async fn desk_windows(desk: State<'_, DeskService>) -> AppResult<Vec<DeskWindow>> {
    let desk = desk.inner().clone();
    tauri::async_runtime::spawn_blocking(move || desk.windows())
        .await
        .map_err(|e| AppError::Platform(e.to_string()))
}

#[tauri::command]
pub fn desk_set_on_top(id: i64, on: bool, desk: State<'_, DeskService>) -> AppResult<()> {
    desk.set_on_top(id, on)
}

/// 20–100; 100 makes it solid again.
#[tauri::command]
pub fn desk_set_opacity(id: i64, percent: u8, desk: State<'_, DeskService>) -> AppResult<()> {
    desk.set_opacity(id, percent)
}

#[tauri::command]
pub fn desk_focus(id: i64, desk: State<'_, DeskService>) -> AppResult<()> {
    desk.focus(id)
}

/// Draw a live view of window `id` into this island at the given physical-pixel rectangle.
#[tauri::command]
pub fn desk_mini_show(
    id: i64,
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
    window: WebviewWindow,
    desk: State<'_, DeskService>,
) -> AppResult<()> {
    if right <= left || bottom <= top || right - left > 4000 || bottom - top > 4000 {
        return Err(AppError::InvalidInput("invalid area".into()));
    }
    desk.mini_show(&window, id, (left, top, right, bottom))
}

#[tauri::command]
pub fn desk_mini_hide(window: WebviewWindow, desk: State<'_, DeskService>) {
    desk.mini_hide(&window);
}

#[tauri::command]
pub async fn routine_run(id: String, desk: State<'_, DeskService>) -> AppResult<usize> {
    let desk = desk.inner().clone();
    tauri::async_runtime::spawn_blocking(move || desk.run(&id, false))
        .await
        .map_err(|e| AppError::Platform(e.to_string()))?
}

/// Pick a program or shortcut for a routine step.
#[tauri::command]
pub async fn routine_pick_app(app: AppHandle) -> AppResult<Option<String>> {
    use tauri_plugin_dialog::DialogExt;
    let picked = tauri::async_runtime::spawn_blocking(move || {
        app.dialog()
            .file()
            .set_title("Choose a program or shortcut")
            .add_filter("Programs and shortcuts", &["exe", "lnk", "url", "bat", "cmd"])
            .blocking_pick_file()
    })
    .await
    .map_err(|e| AppError::Platform(e.to_string()))?;
    Ok(picked
        .and_then(|p| p.into_path().ok())
        .map(|p| p.to_string_lossy().into_owned()))
}

#[tauri::command]
pub async fn routine_pick_folder(app: AppHandle) -> AppResult<Option<String>> {
    use tauri_plugin_dialog::DialogExt;
    let picked = tauri::async_runtime::spawn_blocking(move || {
        app.dialog()
            .file()
            .set_title("Choose a folder")
            .blocking_pick_folder()
    })
    .await
    .map_err(|e| AppError::Platform(e.to_string()))?;
    Ok(picked
        .and_then(|p| p.into_path().ok())
        .map(|p| p.to_string_lossy().into_owned()))
}
