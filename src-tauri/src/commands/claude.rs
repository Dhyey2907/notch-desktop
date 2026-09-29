use tauri::State;

use crate::{
    claude::{
        service::ClaudeService,
        usage::{UsageRange, UsageSummary},
        ClaudeStatus,
    },
    error::AppResult,
};

#[tauri::command]
pub fn claude_status(claude: State<'_, ClaudeService>) -> ClaudeStatus {
    claude.status()
}

/// Local usage estimate for a range, computed from Claude Code's transcripts on this PC.
#[tauri::command]
pub async fn claude_usage(range: UsageRange, claude: State<'_, ClaudeService>) -> AppResult<UsageSummary> {
    claude.usage(range)
}

/// Exactly what connecting adds to Claude Code's settings (token masked), for confirmation.
#[tauri::command]
pub fn claude_hooks_preview(claude: State<'_, ClaudeService>) -> String {
    claude.preview()
}

#[tauri::command]
pub fn claude_connect(claude: State<'_, ClaudeService>) -> AppResult<()> {
    claude.connect()
}

#[tauri::command]
pub fn claude_disconnect(claude: State<'_, ClaudeService>) -> AppResult<()> {
    claude.disconnect()
}
