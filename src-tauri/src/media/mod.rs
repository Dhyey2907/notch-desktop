//! Media module backend (architecture.md §7 Media, plan.md Phase 6).
//!
//! The `MediaPort` trait isolates the Windows media-session API behind a small interface;
//! `MediaService` owns the latest state and publishes `MediaStateChanged` events. The UI
//! never touches Windows APIs directly (plan.md Rule 4).

#[cfg(debug_assertions)]
pub mod mock;
#[cfg(windows)]
pub mod windows;

use std::sync::{Arc, Mutex};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::{
    error::AppResult,
    events::{AppEvent, EventBus, EventEnvelope},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export)]
pub enum PlaybackStatus {
    Playing,
    Paused,
    Stopped,
    /// Opening, changing track, or a state the source doesn't report.
    Other,
}

/// What the active source supports. Unsupported controls are shown disabled, never faked.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct MediaCapabilities {
    pub play_pause: bool,
    pub next: bool,
    pub previous: bool,
    pub seek: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct MediaState {
    /// Friendly name of the app that owns the session, e.g. "Spotify".
    pub source_app: String,
    pub title: String,
    pub artist: String,
    pub album: String,
    /// Absolute path of cached artwork, served to the webview via the asset protocol.
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub artwork_path: Option<String>,
    pub status: PlaybackStatus,
    // Milliseconds fit comfortably in a JS number; ts-rs would otherwise emit `bigint`.
    #[ts(optional, type = "number")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position_ms: Option<u64>,
    #[ts(optional, type = "number")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
    /// When `position_ms` was sampled; the UI interpolates from here instead of polling.
    pub position_updated_at: DateTime<Utc>,
    pub rate: f64,
    pub capabilities: MediaCapabilities,
    /// Development mock data; the UI badges it so it can't be mistaken for real playback.
    pub mock: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Deserialize, TS)]
#[serde(tag = "kind", rename_all = "camelCase")]
#[ts(export)]
pub enum MediaCommand {
    PlayPause,
    Next,
    Previous,
    Seek {
        #[serde(rename = "positionMs")]
        #[ts(type = "number")]
        position_ms: u64,
    },
}

/// Receives every state change from an adapter (`None` = no active session).
pub type MediaSink = Arc<dyn Fn(Option<MediaState>) + Send + Sync>;

/// Capability-oriented port for a media-session provider (architecture.md §8).
pub trait MediaPort: Send + Sync {
    /// Begin watching sessions; the adapter calls `sink` on every change. Non-blocking.
    fn start(&self, sink: MediaSink) -> AppResult<()>;
    fn command(&self, command: MediaCommand) -> AppResult<()>;
}

pub struct MediaService {
    port: Box<dyn MediaPort>,
    latest: Arc<Mutex<Option<MediaState>>>,
}

impl MediaService {
    pub fn new(port: Box<dyn MediaPort>) -> Self {
        Self {
            port,
            latest: Arc::new(Mutex::new(None)),
        }
    }

    /// Start the adapter, publishing each change on the bus. A failure here is logged and
    /// leaves the module in its "unavailable" state; it never blocks the island.
    pub fn start(&self, publish: impl Fn(EventEnvelope) + Send + Sync + 'static) {
        let latest = self.latest.clone();
        let sink: MediaSink = Arc::new(move |state| {
            *latest.lock().unwrap_or_else(|e| e.into_inner()) = state.clone();
            publish(
                EventEnvelope::new("media", AppEvent::MediaStateChanged { state })
                    .with_dedup_key("media-state"),
            );
        });
        if let Err(error) = self.port.start(sink) {
            tracing::warn!(%error, "media adapter failed to start");
        }
    }

    pub fn latest(&self) -> Option<MediaState> {
        self.latest.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }

    pub fn command(&self, command: MediaCommand) -> AppResult<()> {
        self.port.command(command)
    }
}

/// Publisher that forwards envelopes onto the app's event bus.
pub fn bus_publisher(app: tauri::AppHandle) -> impl Fn(EventEnvelope) + Send + Sync + 'static {
    use tauri::Manager;
    move |envelope| {
        let state = app.state::<crate::state::AppState>();
        let bus: &EventBus = &state.bus;
        bus.publish(envelope);
    }
}

/// The raw id of the app whose media session is current ("Chrome", "MSEdge",
/// "SpotifyAB.SpotifyMusic_…!Spotify"); "" when there is none.
static PLAYING_SOURCE: std::sync::Mutex<String> = std::sync::Mutex::new(String::new());

pub fn set_playing_source(app_user_model_id: &str) {
    let mut source = PLAYING_SOURCE.lock().unwrap_or_else(|e| e.into_inner());
    if *source != app_user_model_id {
        app_user_model_id.clone_into(&mut source);
    }
}

pub fn playing_source() -> String {
    PLAYING_SOURCE.lock().unwrap_or_else(|e| e.into_inner()).clone()
}

/// Human-friendly app name from a Windows app user model id (AUMID) or executable, e.g.
/// `SpotifyAB.SpotifyMusic_zpdnekdrzrea0!Spotify` → `Spotify`, `chrome.exe` → `Chrome`,
/// `4DF9E0F8.Netflix_mcm4njqhnhss8!Netflix.App` → `Netflix`.
pub fn display_name(app_user_model_id: &str) -> String {
    // Store apps whose internal names differ from what users see.
    const KNOWN: [(&str, &str); 4] = [
        ("Microsoft.ZuneMusic", "Media Player"),
        ("Microsoft.ZuneVideo", "Films & TV"),
        ("Microsoft.WindowsSoundRecorder", "Sound Recorder"),
        ("MSEdge", "Microsoft Edge"),
    ];

    // Sites installed as apps (or opened in app mode) play under the browser's own id plus a
    // generated suffix: "MSEdge.localhost_x7k2…", "Chrome._crx_abc…". Name the browser.
    for (prefix, browser) in [
        ("MSEdge.", "Microsoft Edge"),
        ("Chrome.", "Google Chrome"),
        ("Brave.", "Brave"),
    ] {
        if app_user_model_id.starts_with(prefix) {
            return browser.to_owned();
        }
    }

    let (package, app) = match app_user_model_id.split_once('!') {
        Some((package, app)) => (Some(package), app),
        None => (None, app_user_model_id),
    };
    // "Contoso.Player_8wekyb3d8bbwe" → "Contoso.Player"
    let family = package.map(|p| p.split('_').next().unwrap_or(p));
    if let Some((_, friendly)) = KNOWN.iter().find(|(id, _)| family == Some(*id) || app == *id) {
        return (*friendly).to_owned();
    }

    let app = app.rsplit(['\\', '/']).next().unwrap_or(app);
    let app = app
        .strip_suffix(".exe")
        .or_else(|| app.strip_suffix(".EXE"))
        .unwrap_or(app);
    let app = app.strip_suffix(".App").unwrap_or(app);
    // Generic entry points ("App") say nothing; use the package's last segment instead.
    let name = if app.eq_ignore_ascii_case("app") || app.is_empty() {
        family.and_then(|f| f.rsplit('.').next()).unwrap_or(app)
    } else {
        app
    };

    let mut chars = name.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => "Unknown app".to_owned(),
    }
}

/// Windows `TimeSpan` ticks (100 ns) to milliseconds; negative values clamp to zero.
pub fn ticks_to_ms(ticks: i64) -> u64 {
    (ticks.max(0) / 10_000) as u64
}

/// Milliseconds to `TimeSpan` ticks.
pub fn ms_to_ticks(ms: u64) -> i64 {
    (ms as i64).saturating_mul(10_000)
}

/// Windows `DateTime` (100 ns ticks since 1601-01-01 UTC) to `chrono`. `None` for unset (0).
pub fn filetime_to_utc(universal_time: i64) -> Option<DateTime<Utc>> {
    const UNIX_EPOCH_AS_FILETIME: i64 = 116_444_736_000_000_000;
    if universal_time <= 0 {
        return None;
    }
    DateTime::from_timestamp_millis((universal_time - UNIX_EPOCH_AS_FILETIME) / 10_000)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn friendly_names_from_app_ids() {
        assert_eq!(
            display_name("SpotifyAB.SpotifyMusic_zpdnekdrzrea0!Spotify"),
            "Spotify"
        );
        assert_eq!(display_name("chrome.exe"), "Chrome");
        assert_eq!(display_name(r"C:\Program Files\VLC\vlc.exe"), "Vlc");
        assert_eq!(
            display_name("Microsoft.ZuneMusic_8wekyb3d8bbwe!Microsoft.ZuneMusic"),
            "Media Player"
        );
        assert_eq!(
            display_name("4DF9E0F8.Netflix_mcm4njqhnhss8!Netflix.App"),
            "Netflix"
        );
        assert_eq!(display_name("Contoso.Player_abc!App"), "Player");
        assert_eq!(display_name(""), "Unknown app");
        assert_eq!(display_name("MSEdge.localhost_kdrop.html_x7k2"), "Microsoft Edge");
        assert_eq!(display_name("Chrome._crx_abcdef"), "Google Chrome");
    }

    #[test]
    fn converts_windows_time_units() {
        assert_eq!(ticks_to_ms(1_234_560_000), 123_456);
        assert_eq!(ticks_to_ms(-5), 0);
        assert_eq!(ms_to_ticks(1_500), 15_000_000);
        // 2026-01-01T00:00:00Z
        let t = filetime_to_utc(134_116_992_000_000_000).unwrap();
        assert_eq!(t.to_rfc3339(), "2026-01-01T00:00:00+00:00");
        assert_eq!(filetime_to_utc(0), None);
    }

    #[test]
    fn commands_deserialize_from_the_frontend_shape() {
        let seek: MediaCommand = serde_json::from_str(r#"{"kind":"seek","positionMs":42000}"#).unwrap();
        assert_eq!(seek, MediaCommand::Seek { position_ms: 42_000 });
        let next: MediaCommand = serde_json::from_str(r#"{"kind":"next"}"#).unwrap();
        assert_eq!(next, MediaCommand::Next);
    }
}
