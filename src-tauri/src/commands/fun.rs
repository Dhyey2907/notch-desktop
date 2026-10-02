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
