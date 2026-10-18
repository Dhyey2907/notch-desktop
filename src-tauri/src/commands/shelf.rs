use tauri::{State, WebviewWindow};

use crate::{
    error::AppResult,
    shelf::{service::ShelfService, ShelfSnapshot},
};

#[tauri::command]
pub fn shelf_snapshot(shelf: State<'_, ShelfService>) -> AppResult<ShelfSnapshot> {
    shelf.snapshot()
}

/// Park files on the shelf; returns how many were new.
#[tauri::command]
pub fn shelf_add(paths: Vec<String>, shelf: State<'_, ShelfService>) -> AppResult<usize> {
    shelf.add(&paths)
}

#[tauri::command]
pub fn shelf_remove(ids: Vec<String>, shelf: State<'_, ShelfService>) -> AppResult<()> {
    shelf.remove(&ids)
}

#[tauri::command]
pub fn shelf_clear(shelf: State<'_, ShelfService>) -> AppResult<()> {
    shelf.clear()
}

/// Copy files to the clipboard (no ids: all of them).
#[tauri::command]
pub fn shelf_copy(ids: Vec<String>, shelf: State<'_, ShelfService>) -> AppResult<()> {
    shelf.copy(&ids)
}

#[tauri::command]
pub fn shelf_reveal(id: String, shelf: State<'_, ShelfService>) -> AppResult<()> {
    shelf.reveal(&id)
}

/// Drag files out of this island (no ids: all of them); true if they were dropped somewhere.
#[tauri::command]
pub async fn shelf_drag(
    ids: Vec<String>,
    window: WebviewWindow,
    shelf: State<'_, ShelfService>,
) -> AppResult<bool> {
    shelf.drag(window, ids).await
}

/// Keep a file on the shelf past the clean-up (or let it go again).
#[tauri::command]
pub fn shelf_pin(id: String, pinned: bool, shelf: State<'_, ShelfService>) -> AppResult<()> {
    shelf.pin(&id, pinned)
}

/// Copy the text in a screenshot (or any picture) to the clipboard; returns the text.
#[tauri::command]
pub async fn screenshot_text(path: String) -> AppResult<String> {
    tauri::async_runtime::spawn_blocking(move || {
        #[cfg(windows)]
        {
            let text = crate::shelf::screenshots::read_text(std::path::Path::new(&path))?;
            if !text.trim().is_empty() {
                crate::clipboard::win::write(crate::clipboard::win::Put::Text(&text))?;
            }
            Ok(text)
        }
        #[cfg(not(windows))]
        {
            let _ = path;
            Ok(String::new())
        }
    })
    .await
    .map_err(|e| crate::error::AppError::Platform(e.to_string()))?
}

/// Open a picture in Paint to mark it up.
#[tauri::command]
pub fn screenshot_annotate(path: String) -> AppResult<()> {
    let p = std::path::Path::new(&path);
    if !p.is_file() || !crate::shelf::screenshots::is_screenshot(p) {
        return Err(crate::error::AppError::InvalidInput(
            "That picture is gone".into(),
        ));
    }
    std::process::Command::new("mspaint.exe")
        .arg(p)
        .spawn()
        .map(|_| ())
        .map_err(|e| crate::error::AppError::Platform(format!("could not open Paint: {e}")))
}
