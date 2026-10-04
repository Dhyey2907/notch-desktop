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

/// Whether an API key is saved for the OpenAI-compatible provider (the key itself never
/// leaves Windows Credential Manager).
#[tauri::command]
pub fn ai_key_saved() -> AppResult<bool> {
    Ok(crate::secrets::get(crate::secrets::Credential::AiApiKey)?.is_some_and(|k| !k.is_empty()))
}

#[tauri::command]
pub fn ai_set_key(key: String) -> AppResult<()> {
    let key = key.trim();
    if key.is_empty() || key.len() > 500 || key.chars().any(char::is_whitespace) {
        return Err(crate::error::AppError::InvalidInput(
            "that doesn't look like an API key".into(),
        ));
    }
    crate::secrets::set(crate::secrets::Credential::AiApiKey, key)
}

#[tauri::command]
pub fn ai_clear_key() -> AppResult<()> {
    crate::secrets::delete(crate::secrets::Credential::AiApiKey)
}

/// A one-line test of the chosen AI (Settings → AI → Test).
#[tauri::command]
pub async fn ai_test(app: tauri::AppHandle, state: State<'_, crate::state::AppState>) -> AppResult<String> {
    use tauri::Manager;
    let ai = state
        .settings
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .ai
        .clone();
    let workdir = app
        .path()
        .app_cache_dir()
        .map_err(|e| crate::error::AppError::Platform(e.to_string()))?
        .join("ask");
    tauri::async_runtime::spawn_blocking(move || {
        crate::ask::providers::answer(
            &ai,
            "You are checking that a connection works. Reply with a short friendly greeting of at most six words.",
            "Hello!",
            &workdir,
        )
    })
    .await
    .map_err(|e| crate::error::AppError::Platform(e.to_string()))?
}

/// Models installed in Ollama (empty if it isn't running).
#[tauri::command]
pub async fn ai_ollama_models() -> Vec<String> {
    crate::ask::providers::ollama_models().await
}
