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

/// Turn the paste stack on (copies from now on queue up for the paste-next shortcut) or off.
#[tauri::command]
pub fn clipboard_stack(on: bool, clipboard: State<'_, ClipboardService>) {
    clipboard.set_stack(on);
}

/// Copy text privately (not in history, not in Windows' clipboard history) and clear it after
/// `clear_secs` unless something else was copied meanwhile. Used for 2FA codes from the phone.
#[tauri::command]
pub fn clipboard_copy_private(text: String, clear_secs: u32) -> AppResult<()> {
    if text.is_empty() || text.len() > 200 {
        return Err(crate::error::AppError::InvalidInput("invalid text".into()));
    }
    #[cfg(windows)]
    {
        use crate::clipboard::win::{clear_if_unchanged, sequence, write, Put};
        write(Put::PrivateText(&text))?;
        let copied = sequence();
        let secs = u64::from(clear_secs.clamp(5, 300));
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_secs(secs));
            clear_if_unchanged(copied);
        });
        Ok(())
    }
    #[cfg(not(windows))]
    {
        let _ = clear_secs;
        Err(crate::error::AppError::Platform("not supported".into()))
    }
}
