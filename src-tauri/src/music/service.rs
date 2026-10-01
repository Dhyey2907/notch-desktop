//! Follows playback: runs the beat capture while music plays (and the user wants it), fetches
//! synced lyrics when the track changes, and serves album-art colors.

use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use serde_json::Value;
use tauri::{AppHandle, Manager};

use super::{parse_lrc, track_key, Lyrics, Palette};
use crate::{
    events::AppEvent,
    media::{MediaState, PlaybackStatus},
    state::AppState,
};

/// Pulses sent to the notch at most this often.
const PULSE_EVERY: Duration = Duration::from_millis(30);
const LYRICS_CACHE: usize = 50;
const USER_AGENT: &str = "Notch-Desktop (https://github.com/Dhyey2907/notch-desktop)";

#[derive(Default)]
struct Inner {
    #[cfg(windows)]
    capture: Option<super::loopback::Capture>,
    track: Option<String>,
    lyrics: Option<Lyrics>,
    cache: HashMap<String, Lyrics>,
    palettes: HashMap<PathBuf, Option<Palette>>,
}

#[derive(Clone)]
pub struct MusicService {
    app: AppHandle,
    artwork_dir: PathBuf,
    http: reqwest::Client,
    inner: Arc<Mutex<Inner>>,
}

impl MusicService {
    pub fn new(app: AppHandle, artwork_dir: PathBuf) -> Self {
        Self {
            app,
            artwork_dir,
            http: reqwest::Client::builder()
                .timeout(Duration::from_secs(10))
                .user_agent(USER_AGENT)
                .build()
                .expect("static client config"),
            inner: Arc::default(),
        }
    }

    fn inner(&self) -> std::sync::MutexGuard<'_, Inner> {
        self.inner.lock().unwrap_or_else(|e| e.into_inner())
    }

    pub fn start(&self) {
        let this = self.clone();
        let mut events = self.app.state::<AppState>().bus.subscribe();
        tauri::async_runtime::spawn(async move {
            let mut media: Option<MediaState> = None;
            loop {
                match events.recv().await {
                    Ok(envelope) => match &envelope.event {
                        AppEvent::MediaStateChanged { state } => {
                            media = state.clone();
                            this.follow(media.as_ref());
                        }
                        AppEvent::SettingsChanged(_) => this.follow(media.as_ref()),
                        _ => {}
                    },
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                    Err(_) => break,
                }
            }
        });
    }

    fn settings(&self) -> crate::settings::MusicSettings {
        self.app
            .state::<AppState>()
            .settings
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .music
            .clone()
    }

    fn follow(&self, media: Option<&MediaState>) {
        let settings = self.settings();
        let playing = media.is_some_and(|m| m.status == PlaybackStatus::Playing && !m.mock);
        self.set_capture(playing && !settings.hide_reactive);

        let key = media
            .filter(|m| !m.title.is_empty())
            .map(|m| track_key(&m.title, &m.artist));
        let changed = {
            let mut inner = self.inner();
            let changed = inner.track != key;
            if changed {
                inner.track = key.clone();
                inner.lyrics = None;
            }
            changed
        };
        if changed {
            self.app
                .state::<AppState>()
                .bus
                .emit("music", AppEvent::LyricsChanged(None));
            if let (Some(media), Some(key), false) = (media, key, settings.hide_lyrics) {
                self.fetch_lyrics(media.clone(), key);
            }
        }
    }

    fn set_capture(&self, on: bool) {
        #[cfg(windows)]
        {
            let mut inner = self.inner();
            if on && inner.capture.is_none() {
                let app = self.app.clone();
                let mut last = Instant::now() - PULSE_EVERY;
                let mut beat_pending = false;
                inner.capture = Some(super::loopback::start(move |level, beat| {
                    beat_pending |= beat;
                    if last.elapsed() >= PULSE_EVERY {
                        last = Instant::now();
                        app.state::<AppState>().bus.emit(
                            "music",
                            AppEvent::MusicPulse {
                                level,
                                beat: std::mem::take(&mut beat_pending),
                            },
                        );
                    }
                }));
            } else if !on && inner.capture.is_some() {
                inner.capture = None; // stops the thread
                drop(inner);
                // Let the notch settle back to rest.
                self.app.state::<AppState>().bus.emit(
                    "music",
                    AppEvent::MusicPulse {
                        level: 0.0,
                        beat: false,
                    },
                );
            }
        }
        #[cfg(not(windows))]
        let _ = on;
    }

    fn fetch_lyrics(&self, media: MediaState, key: String) {
        if let Some(cached) = self.inner().cache.get(&key).cloned() {
            self.publish_lyrics(cached);
            return;
        }
        let this = self.clone();
        tauri::async_runtime::spawn(async move {
            let lines = this.lookup(&media).await;
            let lyrics = Lyrics {
                track: key.clone(),
                lines: lines.unwrap_or_default(),
            };
            {
                let mut inner = this.inner();
                if inner.cache.len() >= LYRICS_CACHE {
                    inner.cache.clear();
                }
                inner.cache.insert(key.clone(), lyrics.clone());
            }
            this.publish_lyrics(lyrics);
        });
    }

    /// LRCLIB: exact match by title, artist, album and duration; only synced lyrics are used.
    async fn lookup(&self, media: &MediaState) -> Option<Vec<super::LyricLine>> {
        let mut query = vec![
            ("track_name", media.title.clone()),
            ("artist_name", media.artist.clone()),
        ];
        if !media.album.is_empty() {
            query.push(("album_name", media.album.clone()));
        }
        if let Some(ms) = media.duration_ms.filter(|ms| *ms > 0) {
            query.push(("duration", (ms / 1000).to_string()));
        }
        let exact = self
            .http
            .get("https://lrclib.net/api/get")
            .query(&query)
            .send()
            .await
            .ok()?;
        if exact.status().is_success() {
            let json: Value = exact.json().await.ok()?;
            if json.get("instrumental").and_then(Value::as_bool) == Some(true) {
                return Some(Vec::new());
            }
            if let Some(synced) = json
                .get("syncedLyrics")
                .and_then(Value::as_str)
                .filter(|s| !s.trim().is_empty())
            {
                return Some(parse_lrc(synced));
            }
        }
        // No exact match (album or length differ): search and take the closest synced version.
        let results: Value = self
            .http
            .get("https://lrclib.net/api/search")
            .query(&query[..2])
            .send()
            .await
            .ok()?
            .json()
            .await
            .ok()?;
        super::pick_search(&results, media.duration_ms).map(|lrc| parse_lrc(&lrc))
    }

    fn publish_lyrics(&self, lyrics: Lyrics) {
        {
            let mut inner = self.inner();
            // The song changed while we were fetching: drop the stale answer.
            if inner.track.as_deref() != Some(lyrics.track.as_str()) {
                return;
            }
            inner.lyrics = Some(lyrics.clone());
        }
        self.app
            .state::<AppState>()
            .bus
            .emit("music", AppEvent::LyricsChanged(Some(lyrics)));
    }

    pub fn lyrics(&self) -> Option<Lyrics> {
        self.inner().lyrics.clone()
    }

    /// Colors of a cached album-art file (only files Notch itself saved).
    pub fn palette(&self, path: &str) -> Option<Palette> {
        let path = std::fs::canonicalize(path).ok()?;
        let dir = std::fs::canonicalize(&self.artwork_dir).ok()?;
        if !path.starts_with(&dir) {
            return None;
        }
        if let Some(cached) = self.inner().palettes.get(&path) {
            return cached.clone();
        }
        let palette = std::fs::read(&path).ok().and_then(|bytes| super::palette(&bytes));
        let mut inner = self.inner();
        if inner.palettes.len() > 100 {
            inner.palettes.clear();
        }
        inner.palettes.insert(path, palette.clone());
        palette
    }
}
