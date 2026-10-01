use tauri::State;

use crate::{
    day::{service::DayService, DayPlan, DayState},
    error::AppResult,
    mail::{service::MailService, MailState},
};

#[tauri::command]
pub fn mail_state(mail: State<'_, MailService>) -> MailState {
    mail.state()
}

#[tauri::command]
pub fn mail_refresh(mail: State<'_, MailService>) {
    mail.refresh();
}

/// Take a message out of the inbox (it stays in All Mail).
#[tauri::command]
pub async fn mail_archive(id: String, mail: State<'_, MailService>) -> AppResult<()> {
    mail.inner().clone().archive(&id).await
}

#[tauri::command]
pub async fn mail_mark_read(id: String, mail: State<'_, MailService>) -> AppResult<()> {
    mail.inner().clone().mark_read(&id).await
}

#[tauri::command]
pub fn day_state(day: State<'_, DayService>) -> DayState {
    day.state()
}

/// Plan the rest of today now (Claude Code).
#[tauri::command]
pub async fn day_plan(day: State<'_, DayService>) -> AppResult<DayPlan> {
    day.inner().clone().plan().await
}

#[tauri::command]
pub fn day_refresh(day: State<'_, DayService>) {
    day.refresh();
}
