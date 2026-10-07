use tauri::State;

use crate::{
    calendar::{service::CalendarService, CalendarSnapshot, InviteResponse, NewEvent, QuickAdded},
    error::AppResult,
};

#[tauri::command]
pub fn calendar_get_snapshot(calendar: State<'_, CalendarService>) -> CalendarSnapshot {
    calendar.snapshot()
}

/// Opens Google sign-in in the browser and waits for it to finish. The secret is stored in
/// Windows Credential Manager; pass an empty secret to reuse the saved one.
#[tauri::command]
pub async fn calendar_connect(
    client_id: String,
    client_secret: String,
    calendar: State<'_, CalendarService>,
) -> AppResult<()> {
    calendar.connect(client_id, client_secret).await
}

#[tauri::command]
pub async fn calendar_disconnect(calendar: State<'_, CalendarService>) -> AppResult<()> {
    calendar.disconnect().await
}

#[tauri::command]
pub fn calendar_sync_now(calendar: State<'_, CalendarService>) {
    calendar.sync_now();
}

#[tauri::command]
pub fn calendar_open_link(url: String, calendar: State<'_, CalendarService>) -> AppResult<()> {
    calendar.open_link(&url)
}

#[tauri::command]
pub async fn calendar_create_event(event: NewEvent, calendar: State<'_, CalendarService>) -> AppResult<()> {
    calendar.create_event(event).await
}

#[tauri::command]
pub async fn calendar_quick_add(text: String, calendar: State<'_, CalendarService>) -> AppResult<QuickAdded> {
    calendar.quick_add(&text).await
}

#[tauri::command]
pub async fn calendar_respond(
    event_id: String,
    answer: InviteResponse,
    calendar: State<'_, CalendarService>,
) -> AppResult<()> {
    calendar.respond(&event_id, answer).await
}
