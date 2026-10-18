use tauri::State;

use crate::{
    claude::{
        approvals::{ApprovalChoice, ApprovalRequest, AuditEntry},
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

/// Permission requests currently waiting for you.
#[tauri::command]
pub fn claude_approvals_pending(claude: State<'_, ClaudeService>) -> Vec<ApprovalRequest> {
    claude.pending_approvals()
}

#[tauri::command]
pub fn claude_approval_decide(
    id: String,
    choice: ApprovalChoice,
    claude: State<'_, ClaudeService>,
) -> AppResult<()> {
    claude.decide(&id, choice)
}

/// Recent decisions made through Notch, newest first.
#[tauri::command]
pub async fn claude_approval_audit(
    limit: u32,
    claude: State<'_, ClaudeService>,
) -> AppResult<Vec<AuditEntry>> {
    claude.approval_audit(limit)
}

/// Exactly what enabling plan limits adds to Claude Code's settings (token masked).
#[tauri::command]
pub fn claude_status_line_preview(claude: State<'_, ClaudeService>) -> String {
    claude.status_line_preview()
}

#[tauri::command]
pub fn claude_status_line_enable(claude: State<'_, ClaudeService>) -> AppResult<()> {
    claude.enable_status_line()
}

#[tauri::command]
pub fn claude_status_line_disable(claude: State<'_, ClaudeService>) -> AppResult<()> {
    claude.disable_status_line()
}

fn agent_of(id: &str) -> AppResult<crate::claude::agents::Agent> {
    crate::claude::agents::Agent::from_id(id)
        .ok_or_else(|| crate::error::AppError::InvalidInput("unknown agent".into()))
}

/// What connecting Codex or Gemini CLI would add to its config.
#[tauri::command]
pub fn agent_preview(agent: String, claude: State<'_, ClaudeService>) -> AppResult<String> {
    claude.agent_preview(agent_of(&agent)?)
}

#[tauri::command]
pub fn agent_connect(agent: String, claude: State<'_, ClaudeService>) -> AppResult<()> {
    claude.agent_connect(agent_of(&agent)?)
}

#[tauri::command]
pub fn agent_disconnect(agent: String, claude: State<'_, ClaudeService>) -> AppResult<()> {
    claude.agent_disconnect(agent_of(&agent)?)
}

/// Codex usage from the session logs Codex keeps on this PC.
#[tauri::command]
pub async fn codex_usage(
    range: crate::claude::usage::UsageRange,
    claude: State<'_, ClaudeService>,
) -> AppResult<crate::claude::codex_usage::CodexUsage> {
    let claude = claude.inner().clone();
    tauri::async_runtime::spawn_blocking(move || claude.codex_usage(range))
        .await
        .map_err(|e| crate::error::AppError::Platform(e.to_string()))
}

/// Send an example approval card (nothing runs, whatever you answer).
#[tauri::command]
pub fn claude_test_approval(claude: State<'_, ClaudeService>) {
    claude.test_approval();
}

/// Tokens and estimated cost per session today.
#[tauri::command]
pub async fn claude_session_usage(
    state: State<'_, crate::state::AppState>,
) -> AppResult<Vec<crate::claude::sessions::SessionUsage>> {
    let day = chrono::Local::now().format("%Y-%m-%d").to_string();
    state
        .db
        .with(|conn| crate::claude::sessions::usage_for_day(conn, &day))
}

/// Bring a live session's terminal window to the front.
#[tauri::command]
pub async fn claude_focus_session(session_id: String, claude: State<'_, ClaudeService>) -> AppResult<()> {
    let pid = claude
        .status()
        .live
        .into_iter()
        .find(|s| s.session_id == session_id)
        .and_then(|s| s.pid)
        .ok_or_else(|| crate::error::AppError::InvalidInput("That session's window isn't known".into()))?;
    tauri::async_runtime::spawn_blocking(move || crate::claude::sessions::focus_process(pid))
        .await
        .map_err(|e| crate::error::AppError::Platform(e.to_string()))?
}

/// Folders Claude Code has worked in, most recent first.
#[tauri::command]
pub fn claude_projects(state: State<'_, crate::state::AppState>) -> AppResult<Vec<String>> {
    state.db.with(|conn| crate::claude::sessions::projects(conn, 30))
}

/// Start Claude Code on a task in a project folder, in a new terminal.
#[tauri::command]
pub fn claude_launch(cwd: String, prompt: String) -> AppResult<()> {
    crate::claude::sessions::launch(&cwd, &prompt)
}
