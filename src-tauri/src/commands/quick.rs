use tauri::{AppHandle, Manager, State};

use crate::{
    error::{AppError, AppResult},
    quick::{parse_guess, sort_input, CaptureGuess, MAX_CHARS, SORT_PROMPT},
    state::AppState,
};

/// Ask the AI chosen in Settings → AI what a quick-capture line is (sends the line to it).
#[tauri::command]
pub async fn capture_sort(
    text: String,
    app: AppHandle,
    state: State<'_, AppState>,
) -> AppResult<CaptureGuess> {
    let text = text.trim().to_owned();
    if text.is_empty() || text.chars().count() > MAX_CHARS {
        return Err(AppError::InvalidInput("Type something first".into()));
    }
    let ai = state
        .settings
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .ai
        .clone();
    let workdir = app
        .path()
        .app_cache_dir()
        .map_err(|e| AppError::Platform(e.to_string()))?
        .join("ask");
    let input = sort_input(&text, chrono::Local::now());
    let answer = tauri::async_runtime::spawn_blocking(move || {
        crate::ask::providers::answer(&ai, SORT_PROMPT, &input, &workdir)
    })
    .await
    .map_err(|e| AppError::Platform(e.to_string()))??;
    parse_guess(&answer, &text, chrono::Utc::now())
}
