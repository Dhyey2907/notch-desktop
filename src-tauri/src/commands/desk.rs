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

/// Snap a window into part of its screen (fractions of the work area).
#[tauri::command]
pub fn desk_snap(id: i64, zone: crate::desk::place::Zone, desk: State<'_, DeskService>) -> AppResult<()> {
    let ok = |v: f64| v.is_finite() && (0.0..=1.0).contains(&v);
    if !(ok(zone.x) && ok(zone.y) && ok(zone.w) && ok(zone.h)) || zone.w <= 0.0 || zone.h <= 0.0 {
        return Err(AppError::InvalidInput("invalid zone".into()));
    }
    desk.snap(id, zone)
}

#[tauri::command]
pub fn workspace_list(desk: State<'_, DeskService>) -> AppResult<Vec<crate::desk::place::Workspace>> {
    desk.workspaces()
}

/// Save where every window is under a name; returns how many windows.
#[tauri::command]
pub async fn workspace_save(name: String, desk: State<'_, DeskService>) -> AppResult<usize> {
    let name = name.trim().to_string();
    if name.is_empty() || name.len() > 60 || name == crate::desk::workspace::LAST_SESSION {
        return Err(AppError::InvalidInput("Give the workspace a short name".into()));
    }
    let desk = desk.inner().clone();
    tauri::async_runtime::spawn_blocking(move || desk.workspace_save(&name))
        .await
        .map_err(|e| AppError::Platform(e.to_string()))?
}

/// Put a workspace back; returns (windows placed, programs started).
#[tauri::command]
pub async fn workspace_restore(name: String, desk: State<'_, DeskService>) -> AppResult<(usize, usize)> {
    let desk = desk.inner().clone();
    tauri::async_runtime::spawn_blocking(move || desk.workspace_restore(&name))
        .await
        .map_err(|e| AppError::Platform(e.to_string()))?
}

#[tauri::command]
pub fn workspace_delete(name: String, desk: State<'_, DeskService>) -> AppResult<()> {
    desk.workspace_delete(&name)
}

/// After a restart: apps from last session that aren't open (asked once per run).
#[tauri::command]
pub async fn workspace_resume_offer(desk: State<'_, DeskService>) -> AppResult<Vec<String>> {
    let desk = desk.inner().clone();
    tauri::async_runtime::spawn_blocking(move || desk.resume_offer())
        .await
        .map_err(|e| AppError::Platform(e.to_string()))?
}

/// One tile of the window switcher.
#[derive(serde::Deserialize)]
pub struct GridTile {
    id: i64,
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
}

/// The window switcher's live views (physical pixels in this island).
#[tauri::command]
pub fn desk_grid_show(
    tiles: Vec<GridTile>,
    window: WebviewWindow,
    desk: State<'_, DeskService>,
) -> AppResult<()> {
    let tiles: Vec<crate::desk::service::GridTile> = tiles
        .into_iter()
        .filter(|t| {
            t.right > t.left && t.bottom > t.top && t.right - t.left < 4000 && t.bottom - t.top < 4000
        })
        .map(|t| (t.id, (t.left, t.top, t.right, t.bottom)))
        .collect();
    desk.grid_show(&window, &tiles)
}

#[tauri::command]
pub fn desk_grid_hide(window: WebviewWindow, desk: State<'_, DeskService>) {
    desk.grid_hide(&window);
}

/// Show window `id` live in a small always-on-top picture-in-picture window.
#[tauri::command]
pub fn pip_open(id: i64, app: AppHandle) -> AppResult<()> {
    crate::desk::pip::open(&app, id)
}

/// What the picture-in-picture window shows.
#[tauri::command]
pub fn pip_source() -> Option<crate::desk::pip::PipSource> {
    crate::desk::pip::source()
}

/// Draw the source into the picture-in-picture window (physical pixels), cropped to fractions
/// of the source; returns the source window's size.
#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub fn pip_place(
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
    crop_x: f64,
    crop_y: f64,
    crop_w: f64,
    crop_h: f64,
    dim: bool,
    app: AppHandle,
) -> AppResult<(i32, i32)> {
    let ok = |v: f64| v.is_finite() && (0.0..=1.0).contains(&v);
    if right <= left || bottom <= top || !(ok(crop_x) && ok(crop_y) && ok(crop_w) && ok(crop_h)) {
        return Err(AppError::InvalidInput("invalid area".into()));
    }
    crate::desk::pip::place(
        &app,
        (left, top, right, bottom),
        (crop_x, crop_y, crop_w, crop_h),
        dim,
    )
}

#[tauri::command]
pub fn pip_close(app: AppHandle) {
    crate::desk::pip::close(&app);
}

/// Don't offer this habit routine again.
#[tauri::command]
pub fn habit_dismiss(key: String, state: State<'_, crate::state::AppState>) -> AppResult<()> {
    if key.is_empty() || key.len() > 400 {
        return Err(AppError::InvalidInput("invalid key".into()));
    }
    state.db.with(|c| crate::habits::dismiss(c, &key))
}

fn change_stickies(
    app: &AppHandle,
    change: impl FnOnce(&mut Vec<crate::desk::sticky::Sticky>),
) -> AppResult<()> {
    use tauri::Manager;
    let state = app.state::<crate::state::AppState>();
    let mut next = state.settings.lock().unwrap_or_else(|e| e.into_inner()).clone();
    change(&mut next.desk.stickies);
    crate::commands::settings::persist_and_broadcast(&state, next).map(|_| ())
}

#[tauri::command]
pub fn sticky_list(state: State<'_, crate::state::AppState>) -> Vec<crate::desk::sticky::Sticky> {
    state
        .settings
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .desk
        .stickies
        .clone()
}

/// Pin a note to the desktop (or bring its sticky forward).
#[tauri::command]
pub fn sticky_pin(note_id: String, app: AppHandle) -> AppResult<()> {
    use crate::desk::sticky;
    use tauri::Manager;
    if !sticky::valid_id(&note_id) {
        return Err(AppError::InvalidInput("invalid note".into()));
    }
    let existing = sticky_list(app.state())
        .into_iter()
        .find(|s| s.note_id == note_id);
    let s = match existing {
        Some(s) => s,
        None => {
            let count = sticky_list(app.state()).len();
            if count >= sticky::MAX_STICKIES {
                return Err(AppError::InvalidInput(format!(
                    "Up to {} notes on the desktop",
                    sticky::MAX_STICKIES
                )));
            }
            let monitor = app
                .primary_monitor()?
                .ok_or_else(|| AppError::Platform("no screen".into()))?;
            let area = monitor.work_area();
            let (x, y, w, h) = sticky::place_new(
                (
                    area.position.x,
                    area.position.y,
                    area.size.width,
                    area.size.height,
                ),
                monitor.scale_factor(),
                count,
            );
            let s = sticky::Sticky {
                note_id: note_id.clone(),
                x,
                y,
                w,
                h,
                color: "yellow".into(),
            };
            let added = s.clone();
            change_stickies(&app, move |list| list.push(added))?;
            s
        }
    };
    sticky::show(&app, &s)
}

/// The sticky was moved or resized.
#[tauri::command]
pub fn sticky_place(note_id: String, x: i32, y: i32, w: u32, h: u32, app: AppHandle) -> AppResult<()> {
    change_stickies(&app, |list| {
        if let Some(s) = list.iter_mut().find(|s| s.note_id == note_id) {
            s.x = x;
            s.y = y;
            s.w = w.clamp(120, 1600);
            s.h = h.clamp(100, 1600);
        }
    })
}

#[tauri::command]
pub fn sticky_color(note_id: String, color: String, app: AppHandle) -> AppResult<()> {
    if !crate::desk::sticky::COLORS.contains(&color.as_str()) {
        return Err(AppError::InvalidInput("invalid colour".into()));
    }
    change_stickies(&app, |list| {
        if let Some(s) = list.iter_mut().find(|s| s.note_id == note_id) {
            s.color = color;
        }
    })
}

/// Take a sticky off the desktop (the note stays in Notes).
#[tauri::command]
pub fn sticky_unpin(note_id: String, app: AppHandle) -> AppResult<()> {
    crate::desk::sticky::close(&app, &note_id);
    change_stickies(&app, |list| list.retain(|s| s.note_id != note_id))
}
