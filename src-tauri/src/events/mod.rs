//! Typed application events (architecture.md §6).
//!
//! Adapters and services publish `AppEvent`s through the `EventBus`. Each event is
//! wrapped in a versioned `EventEnvelope`, forwarded to the webview on the
//! `app-event` channel, and broadcast to in-process Rust subscribers.

use std::{
    collections::HashMap,
    hash::{DefaultHasher, Hash, Hasher},
    sync::Mutex,
    time::{Duration, Instant},
};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;
use ts_rs::TS;

use crate::{
    ask::service::AskSnapshot,
    bridge::BridgeState,
    calendar::{CalendarEvent, CalendarSnapshot},
    claude::{approvals::ApprovalRequest, ClaudeStatus},
    clipboard::{ClipboardSnapshot, CopySuggestion},
    control::{ControlSnapshot, SystemAlert},
    day::{Assignment, DayState},
    developer::DevServer,
    expander::Snippet,
    files::{DownloadDone, DownloadProgress, RecorderState},
    focus::FocusState,
    game::GameHud,
    git::GitSnapshot,
    github::{notifications::GithubNotification, GithubStatus},
    mail::{MailMessage, MailState},
    media::MediaState,
    music::Lyrics,
    phone::{PhoneNotification, PhoneSnapshot},
    settings::Settings,
    shelf::ShelfSnapshot,
    spotify::SpotifySnapshot,
    system::{BluetoothDevice, Desktops, ScreenTime, SoundState, Weather},
    todo::{Task, TodoSnapshot},
    vault::VaultState,
};

/// Name of the single Tauri channel that carries every `EventEnvelope` to the frontend.
pub const APP_EVENT_CHANNEL: &str = "app-event";

/// Bump when an existing event's payload changes shape incompatibly.
pub const EVENT_SCHEMA_VERSION: u32 = 1;

/// Identical events with the same dedup key inside this window are dropped, absorbing
/// bursts from chatty sources (e.g. media sessions firing several change callbacks).
const DEDUP_WINDOW: Duration = Duration::from_millis(750);

/// How urgently an activity should claim the island (plan.md Phase 5).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export)]
#[cfg_attr(not(debug_assertions), allow(dead_code))] // release producers arrive with approvals (M9)
pub enum ActivityPriority {
    Critical,
    High,
    Normal,
    Low,
}

/// Development-only fake activity, used to exercise the activity engine end to end
/// without any real integration (plan.md Phase 4 acceptance). Debug builds only.
#[cfg(debug_assertions)]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct DevTestActivity {
    pub priority: ActivityPriority,
    pub title: String,
    pub body: String,
    #[ts(optional)]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dedup_key: Option<String>,
    #[serde(default)]
    pub persistent: bool,
    #[serde(default)]
    pub actionable: bool,
}

// Settings are the largest payload and are sent rarely; boxing them isn't worth the churn.
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, Serialize, TS)]
#[serde(tag = "type", content = "payload")]
#[ts(export)]
pub enum AppEvent {
    /// Backend finished startup (DB migrated, services initialized).
    AppReady { version: String },
    /// Reply to `diagnostics_ping`; proves the command → event round trip.
    DiagnosticsPong { nonce: String },
    /// The global shortcut asked the island to toggle between collapsed and expanded.
    /// Ctrl+Alt+N: toggle the island with this window label (the one on the pointer's screen).
    ShellToggleRequested { label: String },
    /// A global shortcut asked island `label` to open straight to `module` (Ctrl+Alt+V).
    ShellOpenModuleRequested { label: String, module: String },
    /// Settings were saved; every window replaces its copy with this one.
    SettingsChanged(Settings),
    /// The active media session changed (`None` when nothing is playing).
    MediaStateChanged { state: Option<MediaState> },
    /// A fullscreen app opened or closed on the screen of island `label`.
    FullscreenChanged { label: String, active: bool },
    /// Calendar status or events changed.
    CalendarChanged(CalendarSnapshot),
    /// A meeting is about to start (reminder lead time reached).
    ReminderDue { event: CalendarEvent },
    /// Spotify connection or playlists changed.
    SpotifyChanged(SpotifySnapshot),
    /// GitHub sign-in changed (signed in/out, or a code is waiting for approval).
    GithubChanged(GithubStatus),
    /// Unread GitHub notifications changed (newest first).
    GithubNotificationsChanged(Vec<GithubNotification>),
    /// A new review request, CI result, mention or assignment arrived on GitHub.
    GithubAlert(GithubNotification),
    /// Repository status, history, or a running operation changed.
    GitChanged(GitSnapshot),
    /// Live Claude Code activity, hook connection, or index status changed.
    ClaudeStatusChanged(ClaudeStatus),
    /// New transcript data was indexed; views re-query the usage range they show.
    ClaudeUsageChanged,
    /// Phone Link notifications in Action Center (or access to them) changed.
    PhoneChanged(PhoneSnapshot),
    /// A new phone notification arrived through Phone Link.
    PhoneNotification(PhoneNotification),
    /// A copied date/time or address worth acting on ("Remind me", "Open in Maps").
    CopySuggestion(CopySuggestion),
    /// Clipboard history or pinned clips changed.
    ClipboardChanged(ClipboardSnapshot),
    /// Speaker volume or mute changed (`switched`: sound moved to another device).
    SoundChanged { state: SoundState, switched: bool },
    /// Laptop screen brightness changed (percent).
    BrightnessChanged { level: u8 },
    /// Active screen time today changed (another minute counted).
    ScreenTimeChanged(ScreenTime),
    /// Windows virtual desktops were added, removed, renamed or switched.
    DesktopsChanged(Desktops),
    /// New weather for the Home dashboard.
    WeatherChanged(Weather),
    /// Loudness of what's playing (0–1) and whether a beat just hit; ~30 per second while
    /// music plays. Never logged.
    MusicPulse { level: f32, beat: bool },
    /// Synced lyrics for the current song (`None` while looking them up or between songs).
    LyricsChanged(Option<Lyrics>),
    /// The Ask conversation changed (a question asked, an answer finished, cleared).
    AskChanged(AskSnapshot),
    /// More of an answer arrived. Never logged.
    AskDelta { id: String, text: String },
    /// A focus session started, stopped or finished.
    FocusChanged(FocusState),
    /// A focus session ran its full length.
    FocusDone,
    /// The phone bridge started, changed or ended (`None`: not running).
    BridgeChanged(Option<BridgeState>),
    /// A file arrived from the phone (saved to Downloads\Notch).
    BridgeReceived { name: String },
    /// Game mode readings each second while playing; `None` when the game ends.
    GameHud(Option<GameHud>),
    /// An app went fullscreen that might be a game: ask once.
    GameAsk { exe: String, app: String },
    /// Network, power, radios or accessories changed (Control center).
    ControlChanged(ControlSnapshot),
    /// Something worth a pop-up: internet down or back, unplug, a low accessory, lock keys.
    SystemAlert(SystemAlert),
    /// Downloads in progress (empty once they're all done).
    DownloadsChanged(Vec<DownloadProgress>),
    /// A download finished.
    DownloadDone(DownloadDone),
    /// The audio recorder started or stopped.
    RecorderChanged(RecorderState),
    /// A recording was saved (and put on the shelf).
    RecordingSaved { name: String, path: String },
    /// Vault logins changed (here or in Obsidian).
    VaultChanged(VaultState),
    /// Tracking was removed from a copied link.
    LinkCleaned { removed: u32 },
    /// Text snippets changed.
    SnippetsChanged(Vec<Snippet>),
    /// Text was read from the screen and copied (empty if none was found).
    TextGrabbed { text: String },
    /// Local dev servers changed.
    DevServersChanged(Vec<DevServer>),
    /// A dev server that had been running stopped.
    DevServerStopped { port: u16, name: String },
    /// Gmail glance changed.
    MailChanged(MailState),
    /// New unread mail arrived.
    NewMail(MailMessage),
    /// The day plan or deadlines changed.
    DayChanged(DayState),
    /// The morning plan was made by itself.
    DayPlanReady { first_line: String },
    /// A Classroom assignment is due within a day.
    AssignmentDue(Assignment),
    /// A paired Bluetooth device connected or disconnected.
    BluetoothChanged(BluetoothDevice),
    /// Files on the drop shelf changed.
    ShelfChanged(ShelfSnapshot),
    /// Tasks, notes, or finished-timer alerts changed.
    TodoChanged(TodoSnapshot),
    /// A task's timer ran out (`late`: it ran out while Notch wasn't running).
    TimerDone { task: Task, late: bool },
    /// Permission requests waiting for you changed (all pending, oldest first).
    ApprovalsChanged(Vec<ApprovalRequest>),
    /// Fake activity fired from the dev panel (debug builds only).
    #[cfg(debug_assertions)]
    DevTestActivity(DevTestActivity),
}

impl AppEvent {
    /// Stable name for diagnostics. Logging uses this, never the payload.
    pub fn name(&self) -> &'static str {
        match self {
            AppEvent::AppReady { .. } => "AppReady",
            AppEvent::DiagnosticsPong { .. } => "DiagnosticsPong",
            AppEvent::ShellToggleRequested { .. } => "ShellToggleRequested",
            AppEvent::ShellOpenModuleRequested { .. } => "ShellOpenModuleRequested",
            AppEvent::ClipboardChanged(_) => "ClipboardChanged",
            AppEvent::CopySuggestion(_) => "CopySuggestion",
            AppEvent::ShelfChanged(_) => "ShelfChanged",
            AppEvent::SoundChanged { .. } => "SoundChanged",
            AppEvent::BrightnessChanged { .. } => "BrightnessChanged",
            AppEvent::BluetoothChanged(_) => "BluetoothChanged",
            AppEvent::MailChanged(_) => "MailChanged",
            AppEvent::NewMail(_) => "NewMail",
            AppEvent::DayChanged(_) => "DayChanged",
            AppEvent::DayPlanReady { .. } => "DayPlanReady",
            AppEvent::AssignmentDue(_) => "AssignmentDue",
            AppEvent::DevServersChanged(_) => "DevServersChanged",
            AppEvent::DevServerStopped { .. } => "DevServerStopped",
            AppEvent::VaultChanged(_) => "VaultChanged",
            AppEvent::LinkCleaned { .. } => "LinkCleaned",
            AppEvent::SnippetsChanged(_) => "SnippetsChanged",
            AppEvent::TextGrabbed { .. } => "TextGrabbed",
            AppEvent::DownloadsChanged(_) => "DownloadsChanged",
            AppEvent::DownloadDone(_) => "DownloadDone",
            AppEvent::RecorderChanged(_) => "RecorderChanged",
            AppEvent::RecordingSaved { .. } => "RecordingSaved",
            AppEvent::ControlChanged(_) => "ControlChanged",
            AppEvent::SystemAlert(_) => "SystemAlert",
            AppEvent::GameHud(_) => "GameHud",
            AppEvent::GameAsk { .. } => "GameAsk",
            AppEvent::BridgeChanged(_) => "BridgeChanged",
            AppEvent::BridgeReceived { .. } => "BridgeReceived",
            AppEvent::FocusChanged(_) => "FocusChanged",
            AppEvent::FocusDone => "FocusDone",
            AppEvent::AskChanged(_) => "AskChanged",
            AppEvent::AskDelta { .. } => "AskDelta",
            AppEvent::MusicPulse { .. } => "MusicPulse",
            AppEvent::LyricsChanged(_) => "LyricsChanged",
            AppEvent::ScreenTimeChanged(_) => "ScreenTimeChanged",
            AppEvent::DesktopsChanged(_) => "DesktopsChanged",
            AppEvent::WeatherChanged(_) => "WeatherChanged",
            AppEvent::SettingsChanged(_) => "SettingsChanged",
            AppEvent::MediaStateChanged { .. } => "MediaStateChanged",
            AppEvent::FullscreenChanged { .. } => "FullscreenChanged",
            AppEvent::CalendarChanged(_) => "CalendarChanged",
            AppEvent::ReminderDue { .. } => "ReminderDue",
            AppEvent::SpotifyChanged(_) => "SpotifyChanged",
            AppEvent::GitChanged(_) => "GitChanged",
            AppEvent::GithubChanged(_) => "GithubChanged",
            AppEvent::GithubNotificationsChanged(_) => "GithubNotificationsChanged",
            AppEvent::GithubAlert(_) => "GithubAlert",
            AppEvent::ClaudeStatusChanged(_) => "ClaudeStatusChanged",
            AppEvent::ClaudeUsageChanged => "ClaudeUsageChanged",
            AppEvent::ApprovalsChanged(_) => "ApprovalsChanged",
            AppEvent::TodoChanged(_) => "TodoChanged",
            AppEvent::PhoneChanged(_) => "PhoneChanged",
            AppEvent::PhoneNotification(_) => "PhoneNotification",
            AppEvent::TimerDone { .. } => "TimerDone",
            #[cfg(debug_assertions)]
            AppEvent::DevTestActivity(_) => "DevTestActivity",
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

    #[cfg_attr(not(debug_assertions), allow(dead_code))] // release callers arrive with Media (M5)
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
    dedup_window: Duration,
    /// dedup key -> (last published at, payload fingerprint)
    recent: Mutex<HashMap<String, (Instant, u64)>>,
}

impl EventBus {
    pub fn new(emit_to_frontend: Emitter) -> Self {
        Self::with_dedup_window(emit_to_frontend, DEDUP_WINDOW)
    }

    fn with_dedup_window(emit_to_frontend: Emitter, dedup_window: Duration) -> Self {
        let (tx, _) = broadcast::channel(256);
        Self {
            emit_to_frontend,
            tx,
            dedup_window,
            recent: Mutex::new(HashMap::new()),
        }
    }

    /// True when an identical event with the same dedup key was published within the window.
    fn is_duplicate(&self, envelope: &EventEnvelope) -> bool {
        let Some(key) = &envelope.dedup_key else {
            return false;
        };
        let mut hasher = DefaultHasher::new();
        serde_json::to_string(&envelope.event)
            .unwrap_or_default()
            .hash(&mut hasher);
        let fingerprint = hasher.finish();
        let now = Instant::now();

        let mut recent = self.recent.lock().unwrap_or_else(|e| e.into_inner());
        recent.retain(|_, (at, _)| now.duration_since(*at) < self.dedup_window);
        let duplicate = recent
            .get(key)
            .is_some_and(|(_, previous)| *previous == fingerprint);
        recent.insert(key.clone(), (now, fingerprint));
        duplicate
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
        if self.is_duplicate(&envelope) {
            tracing::trace!(event = envelope.event.name(), "duplicate event suppressed");
            return;
        }
        if !matches!(
            envelope.event,
            AppEvent::MusicPulse { .. }
                | AppEvent::AskDelta { .. }
                | AppEvent::GameHud(_)
                | AppEvent::DownloadsChanged(_)
        ) {
            tracing::debug!(
                id = %envelope.id,
                event = envelope.event.name(),
                source = %envelope.source,
                correlation_id = envelope.correlation_id.as_deref(),
                "event published"
            );
        }
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

    fn counting_bus(window: Duration) -> (EventBus, Arc<Mutex<usize>>) {
        let count = Arc::new(Mutex::new(0));
        let count_clone = count.clone();
        let bus = EventBus::with_dedup_window(Box::new(move |_| *count_clone.lock().unwrap() += 1), window);
        (bus, count)
    }

    fn pong(nonce: &str, key: Option<&str>) -> EventEnvelope {
        let envelope = EventEnvelope::new("test", AppEvent::DiagnosticsPong { nonce: nonce.into() });
        match key {
            Some(key) => envelope.with_dedup_key(key),
            None => envelope,
        }
    }

    #[test]
    fn suppresses_identical_events_with_the_same_dedup_key() {
        let (bus, count) = counting_bus(Duration::from_secs(60));
        bus.publish(pong("a", Some("k")));
        bus.publish(pong("a", Some("k")));
        assert_eq!(*count.lock().unwrap(), 1);
    }

    #[test]
    fn delivers_changed_payloads_and_unkeyed_events() {
        let (bus, count) = counting_bus(Duration::from_secs(60));
        bus.publish(pong("a", Some("k")));
        bus.publish(pong("b", Some("k"))); // same key, new content: a replacement, not a dup
        bus.publish(pong("a", None));
        bus.publish(pong("a", None));
        assert_eq!(*count.lock().unwrap(), 4);
    }

    #[test]
    fn delivers_repeats_after_the_window() {
        let (bus, count) = counting_bus(Duration::ZERO);
        bus.publish(pong("a", Some("k")));
        bus.publish(pong("a", Some("k")));
        assert_eq!(*count.lock().unwrap(), 2);
    }

    #[cfg(debug_assertions)]
    #[test]
    fn dev_activity_uses_camel_case_and_lowercase_priority() {
        let event = AppEvent::DevTestActivity(DevTestActivity {
            priority: ActivityPriority::Critical,
            title: "t".into(),
            body: "b".into(),
            dedup_key: Some("k".into()),
            persistent: true,
            actionable: false,
        });
        let json = serde_json::to_value(&event).unwrap();
        assert_eq!(json["payload"]["priority"], "critical");
        assert_eq!(json["payload"]["dedupKey"], "k");
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
