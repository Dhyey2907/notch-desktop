use tauri::State;

use crate::{
    error::AppResult,
    phone::{service::PhoneService, PhoneSnapshot},
};

/// Phone Link notifications currently in Action Center, plus access status.
#[tauri::command]
pub fn phone_snapshot(phone: State<'_, PhoneService>) -> PhoneSnapshot {
    phone.snapshot()
}

/// Remove a phone notification from Action Center.
#[tauri::command]
pub fn phone_dismiss(id: u32, phone: State<'_, PhoneService>) {
    phone.dismiss(id)
}

/// Open Phone Link (replies happen there).
#[tauri::command]
pub fn phone_open(phone: State<'_, PhoneService>) -> AppResult<()> {
    phone.open_phone_link()
}

/// Open Windows Settings where notification access is granted.
#[tauri::command]
pub fn phone_open_access_settings(phone: State<'_, PhoneService>) -> AppResult<()> {
    phone.open_access_settings()
}
