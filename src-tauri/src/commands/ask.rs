use tauri::State;

use crate::{
    ask::service::{AskService, AskSnapshot},
    error::AppResult,
};

#[tauri::command]
pub fn ask_snapshot(ask: State<'_, AskService>) -> AskSnapshot {
    ask.snapshot()
}

/// Ask Claude Code; the answer streams in as `AskDelta` events. Returns the exchange id.
#[tauri::command]
pub fn ask_question(question: String, include_screen: bool, ask: State<'_, AskService>) -> AppResult<String> {
    ask.ask(&question, include_screen)
}

#[tauri::command]
pub fn ask_stop(ask: State<'_, AskService>) {
    ask.stop();
}

#[tauri::command]
pub fn ask_clear(ask: State<'_, AskService>) {
    ask.clear();
}
