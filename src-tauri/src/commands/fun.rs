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

/// Call the pet out of the notch: it walks the taskbar until it goes back.
#[tauri::command]
pub async fn pet_buddy_show(
    app: tauri::AppHandle,
    fun: State<'_, FunService>,
) -> crate::error::AppResult<()> {
    crate::fun::buddy::show(&app)?;
    fun.wake_pet();
    Ok(())
}

/// Send the pet back: off the taskbar and into the notch.
#[tauri::command]
pub async fn pet_buddy_hide(
    app: tauri::AppHandle,
    fun: State<'_, FunService>,
) -> crate::error::AppResult<()> {
    fun.send_pet_back();
    crate::fun::buddy::hide(&app)
}

/// The walking pet's window takes the mouse while the pointer is on the pet (to pet, carry
/// or throw it) and lets clicks through again afterwards. Async: it touches a window.
#[tauri::command]
pub async fn pet_capture(on: bool, fun: State<'_, FunService>) -> crate::error::AppResult<()> {
    fun.capture_pet(on);
    Ok(())
}

/// The pet is out walking the taskbar.
#[tauri::command]
pub fn pet_walking(fun: State<'_, FunService>) -> bool {
    fun.pet_walking()
}

/// Whether a code editor or terminal is in front, and whether that's on the pet's own screen:
/// "here", "elsewhere" or "none" (the pet gets its laptop out).
#[tauri::command]
pub fn pet_coding_in_front(app: tauri::AppHandle) -> &'static str {
    crate::fun::buddy::coding_in_front(&app)
}

/// Where the app that's playing is, seen from the pet: "here" (on its screen), "elsewhere"
/// (only on another screen) or "unknown". A video on another screen doesn't get the popcorn out.
#[tauri::command]
pub fn pet_media_place(app: tauri::AppHandle) -> &'static str {
    crate::fun::buddy::media_place(&app).as_str()
}
