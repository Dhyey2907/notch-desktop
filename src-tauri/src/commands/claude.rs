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
