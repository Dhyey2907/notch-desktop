use tauri::State;

use crate::{
    error::AppResult,
    history::{self, HistoryEntry},
    state::AppState,
};

/// Remember a card the notch showed (private modules keep only the title).
#[tauri::command]
pub fn history_add(entry: HistoryEntry, state: State<'_, AppState>) -> AppResult<()> {
    let now = chrono::Utc::now().timestamp_millis();
    state.db.with(|conn| history::add(conn, entry, now))
}

/// Cards from the last `hours` (at most 48), newest first.
#[tauri::command]
pub fn history_list(hours: i64, state: State<'_, AppState>) -> AppResult<Vec<HistoryEntry>> {
    let now = chrono::Utc::now().timestamp_millis();
    state.db.with(|conn| history::list(conn, now, hours))
}

#[tauri::command]
pub fn history_clear(state: State<'_, AppState>) -> AppResult<()> {
    state.db.with(history::clear)
}
