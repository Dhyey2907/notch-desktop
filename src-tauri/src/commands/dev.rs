//! Development-only commands. Only registered in debug builds (see `lib.rs`).

use std::time::Duration;

use tauri::State;

use crate::{
    claude::service::ClaudeService,
    events::{AppEvent, DevTestActivity, EventEnvelope},
    state::AppState,
};

/// Longest delay the dev panel may request, so a typo can't park a task for hours.
const MAX_DELAY_MS: u64 = 30_000;

/// Emit a fake activity through the real event pipeline, optionally after a delay so the
/// island can be collapsed first.
#[tauri::command]
pub async fn dev_emit_test_activity(
    activity: DevTestActivity,
    delay_ms: Option<u64>,
    state: State<'_, AppState>,
) -> Result<(), ()> {
    if let Some(delay) = delay_ms {
        tokio::time::sleep(Duration::from_millis(delay.min(MAX_DELAY_MS))).await;
    }
    let mut envelope = EventEnvelope::new("dev", AppEvent::DevTestActivity(activity.clone()));
    if let Some(key) = activity.dedup_key {
        envelope = envelope.with_dedup_key(key);
    }
    state.bus.publish(envelope);
    Ok(())
}

/// Send a fake Claude Code permission request through the real approval flow, so the
/// approval UI can be tested without running Claude Code.
#[tauri::command]
pub async fn dev_emit_test_approval(
    tool: String,
    input: serde_json::Value,
    delay_ms: Option<u64>,
    claude: State<'_, ClaudeService>,
) -> Result<(), ()> {
    if let Some(delay) = delay_ms {
        tokio::time::sleep(Duration::from_millis(delay.min(MAX_DELAY_MS))).await;
    }
    claude.dev_test_approval(tool, input);
    Ok(())
}
