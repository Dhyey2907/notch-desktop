//! Development-only commands. Only registered in debug builds (see `lib.rs`).

use std::time::Duration;

use tauri::State;

use crate::{
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
