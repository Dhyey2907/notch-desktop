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
