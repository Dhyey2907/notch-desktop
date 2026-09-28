//! Typed application events (architecture.md §6).
//!
//! Adapters and services publish `AppEvent`s through the `EventBus`. Each event is
//! wrapped in a versioned `EventEnvelope`, forwarded to the webview on the
//! `app-event` channel, and broadcast to in-process Rust subscribers.

use chrono::{DateTime, Utc};
use serde::Serialize;
use tokio::sync::broadcast;
use ts_rs::TS;

/// Name of the single Tauri channel that carries every `EventEnvelope` to the frontend.
pub const APP_EVENT_CHANNEL: &str = "app-event";

/// Bump when an existing event's payload changes shape incompatibly.
pub const EVENT_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, TS)]
#[serde(tag = "type", content = "payload")]
#[ts(export)]
pub enum AppEvent {
    /// Backend finished startup (DB migrated, services initialized).
    AppReady { version: String },
    /// Reply to `diagnostics_ping`; proves the command → event round trip.
    DiagnosticsPong { nonce: String },
}

impl AppEvent {
    /// Stable name for diagnostics. Logging uses this, never the payload.
    pub fn name(&self) -> &'static str {
        match self {
            AppEvent::AppReady { .. } => "AppReady",
            AppEvent::DiagnosticsPong { .. } => "DiagnosticsPong",
        }
    }
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct EventEnvelope {
    pub id: String,
    pub v: u32,
    pub ts: DateTime<Utc>,
    /// Originating component, e.g. `"app"`, `"media"`, `"git"`.
    pub source: String,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub correlation_id: Option<String>,
    /// Events sharing a dedup key replace each other instead of stacking.
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dedup_key: Option<String>,
    pub event: AppEvent,
}

impl EventEnvelope {
    pub fn new(source: impl Into<String>, event: AppEvent) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            v: EVENT_SCHEMA_VERSION,
            ts: Utc::now(),
            source: source.into(),
            correlation_id: None,
            dedup_key: None,
            event,
        }
    }

    pub fn with_correlation(mut self, id: impl Into<String>) -> Self {
        self.correlation_id = Some(id.into());
        self
    }

    #[allow(dead_code)] // first used by the activity engine (M3)
    pub fn with_dedup_key(mut self, key: impl Into<String>) -> Self {
        self.dedup_key = Some(key.into());
        self
    }
}

/// Delivers envelopes to the webview. Abstracted so the bus is testable without Tauri.
pub type Emitter = Box<dyn Fn(&EventEnvelope) + Send + Sync>;

pub struct EventBus {
    emit_to_frontend: Emitter,
    tx: broadcast::Sender<EventEnvelope>,
}

impl EventBus {
    pub fn new(emit_to_frontend: Emitter) -> Self {
        let (tx, _) = broadcast::channel(256);
        Self { emit_to_frontend, tx }
    }

    pub fn tauri_emitter(app: tauri::AppHandle) -> Emitter {
        use tauri::Emitter as _;
        Box::new(move |envelope| {
            if let Err(error) = app.emit(APP_EVENT_CHANNEL, envelope) {
                tracing::warn!(%error, event = envelope.event.name(), "failed to emit event to webview");
            }
        })
    }

    pub fn publish(&self, envelope: EventEnvelope) {
        tracing::debug!(
            id = %envelope.id,
            event = envelope.event.name(),
            source = %envelope.source,
            correlation_id = envelope.correlation_id.as_deref(),
            "event published"
        );
        (self.emit_to_frontend)(&envelope);
        // No in-process subscribers is normal; ignore the send error.
        let _ = self.tx.send(envelope);
    }

    /// Convenience for the common case of an uncorrelated event.
    pub fn emit(&self, source: &str, event: AppEvent) {
        self.publish(EventEnvelope::new(source, event));
    }

    #[allow(dead_code)] // first used by Rust-side services (M3+)
    pub fn subscribe(&self) -> broadcast::Receiver<EventEnvelope> {
        self.tx.subscribe()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    #[test]
    fn publish_reaches_frontend_and_rust_subscribers() {
        let seen = Arc::new(Mutex::new(Vec::new()));
        let seen_clone = seen.clone();
        let bus = EventBus::new(Box::new(move |e| seen_clone.lock().unwrap().push(e.id.clone())));
        let mut rx = bus.subscribe();

        bus.emit(
            "test",
            AppEvent::AppReady {
                version: "0.0.0".into(),
            },
        );

        assert_eq!(seen.lock().unwrap().len(), 1);
        let received = rx.try_recv().unwrap();
        assert_eq!(received.source, "test");
        assert_eq!(received.v, EVENT_SCHEMA_VERSION);
    }

    #[test]
    fn envelope_serializes_with_tagged_event() {
        let envelope = EventEnvelope::new("app", AppEvent::DiagnosticsPong { nonce: "n1".into() })
            .with_correlation("c1");
        let json = serde_json::to_value(&envelope).unwrap();
        assert_eq!(json["event"]["type"], "DiagnosticsPong");
        assert_eq!(json["event"]["payload"]["nonce"], "n1");
        assert_eq!(json["correlationId"], "c1");
        assert!(json.get("dedupKey").is_none());
    }
}
