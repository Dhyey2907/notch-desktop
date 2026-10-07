use tauri::State;

use crate::{
    error::AppResult,
    readlater::{service::ReadLaterService, ReadItem, ReadLaterSnapshot},
};

#[tauri::command]
pub fn read_later_snapshot(readlater: State<'_, ReadLaterService>) -> AppResult<ReadLaterSnapshot> {
    readlater.snapshot()
}

/// Save a link to read later (the page is fetched once for its title and length).
#[tauri::command]
pub async fn read_later_add(url: String, readlater: State<'_, ReadLaterService>) -> AppResult<ReadItem> {
    readlater.inner().clone().add(&url).await
}

#[tauri::command]
pub fn read_later_set_read(id: String, read: bool, readlater: State<'_, ReadLaterService>) -> AppResult<()> {
    readlater.set_read(&id, read)
}

#[tauri::command]
pub fn read_later_remove(id: String, readlater: State<'_, ReadLaterService>) -> AppResult<()> {
    readlater.remove(&id)
}

#[tauri::command]
pub fn read_later_clear_read(readlater: State<'_, ReadLaterService>) -> AppResult<()> {
    readlater.clear_read()
}

/// Summarize with the AI chosen in Settings → AI (sends the article's text to it).
#[tauri::command]
pub async fn read_later_summarize(id: String, readlater: State<'_, ReadLaterService>) -> AppResult<String> {
    readlater.inner().clone().summarize(&id).await
}
