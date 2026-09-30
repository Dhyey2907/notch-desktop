use tauri::State;

use crate::{
    clipboard::{service::ClipboardService, ClipboardSnapshot},
    error::AppResult,
};

#[tauri::command]
pub fn clipboard_snapshot(clipboard: State<'_, ClipboardService>) -> ClipboardSnapshot {
    clipboard.snapshot()
}

/// Copy a clip again (`formatted`: indent JSON first).
#[tauri::command]
pub fn clipboard_copy(
    id: String,
    formatted: Option<bool>,
    clipboard: State<'_, ClipboardService>,
) -> AppResult<()> {
    clipboard.copy(&id, formatted.unwrap_or(false))
}

/// The full text of a clip, e.g. to save it as a note.
#[tauri::command]
pub fn clipboard_text(id: String, clipboard: State<'_, ClipboardService>) -> AppResult<String> {
    clipboard.text(&id)
}

#[tauri::command]
pub fn clipboard_pin(id: String, clipboard: State<'_, ClipboardService>) -> AppResult<()> {
    clipboard.pin(&id)
}

#[tauri::command]
pub fn clipboard_unpin(id: String, clipboard: State<'_, ClipboardService>) -> AppResult<()> {
    clipboard.unpin(&id)
}

#[tauri::command]
pub fn clipboard_delete(id: String, clipboard: State<'_, ClipboardService>) -> AppResult<()> {
    clipboard.delete(&id)
}

#[tauri::command]
pub fn clipboard_clear(clipboard: State<'_, ClipboardService>) {
    clipboard.clear()
}
