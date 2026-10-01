use tauri::State;

use crate::{
    error::AppResult,
    focus::{service::FocusService, FocusState},
};

#[tauri::command]
pub fn focus_state(focus: State<'_, FocusService>) -> AppResult<FocusState> {
    focus.state()
}

#[tauri::command]
pub fn focus_start(minutes: u32, focus: State<'_, FocusService>) -> AppResult<()> {
    focus.start(minutes)
}

#[tauri::command]
pub fn focus_stop(focus: State<'_, FocusService>) -> AppResult<()> {
    focus.stop()
}
