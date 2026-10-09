use tauri::State;

use crate::fun::{service::FunService, RgbStatus};

/// The album art color while music plays, `None` otherwise (the lights follow it).
#[tauri::command]
pub fn rgb_music_color(hex: Option<String>, fun: State<'_, FunService>) {
    fun.set_music_color(hex);
}

#[tauri::command]
pub fn rgb_status(fun: State<'_, FunService>) -> RgbStatus {
    fun.status()
}

/// Call the pet out of the notch onto the desktop. Async: building a window from a sync
/// command deadlocks on Windows (it runs on the main thread the window needs).
#[tauri::command]
pub async fn pet_buddy_show(app: tauri::AppHandle) -> crate::error::AppResult<()> {
    crate::fun::buddy::show(&app)
}

/// Send the pet back into the notch.
#[tauri::command]
pub async fn pet_buddy_hide(app: tauri::AppHandle) -> crate::error::AppResult<()> {
    crate::fun::buddy::hide(&app)
}

/// Remember where the pet was dropped.
#[tauri::command]
pub async fn pet_buddy_moved(app: tauri::AppHandle) -> crate::error::AppResult<()> {
    crate::fun::buddy::save_position(&app)
}

/// A code editor or terminal is in front (the pet gets its laptop out).
#[tauri::command]
pub fn pet_coding_in_front() -> bool {
    crate::fun::buddy::coding_in_front()
}
