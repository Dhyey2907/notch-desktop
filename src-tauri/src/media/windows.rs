//! Windows Global System Media Transport Controls (GSMTC) adapter.
//!
//! Fully event-driven: we subscribe to the session manager's `CurrentSessionChanged` and the
//! current session's media / playback / timeline change events. There is no polling
//! (plan.md Rule 7); the UI interpolates playback position between timeline updates.

use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, Mutex, OnceLock,
    },
    time::Duration,
};

use chrono::Utc;
use windows::{
    Foundation::TypedEventHandler,
    Media::Control::{
        GlobalSystemMediaTransportControlsSession as Session,
        GlobalSystemMediaTransportControlsSessionManager as Manager,
        GlobalSystemMediaTransportControlsSessionMediaProperties as MediaProperties,
        GlobalSystemMediaTransportControlsSessionPlaybackStatus as WinStatus,
    },
    Storage::Streams::DataReader,
    Win32::System::WinRT::{RoInitialize, RO_INIT_MULTITHREADED},
};

use super::{
    display_name, filetime_to_utc, ms_to_ticks, ticks_to_ms, MediaCapabilities, MediaCommand, MediaPort,
    MediaSink, MediaState, PlaybackStatus,
};
use crate::error::{AppError, AppResult};

/// Largest artwork we will cache; real thumbnails are far smaller.
const MAX_ARTWORK_BYTES: u64 = 5 * 1024 * 1024;
/// Artwork files kept on disk before the oldest are removed.
const MAX_ARTWORK_FILES: usize = 40;
/// A new session often isn't readable for a moment ("device is not ready"). Retry a few
/// times, briefly, instead of reporting "nothing playing" and never looking again.
const READ_RETRIES: u32 = 3;
const READ_RETRY_DELAY: Duration = Duration::from_millis(400);

fn platform(error: windows_core::Error) -> AppError {
    AppError::Platform(error.message())
}

struct Subscription {
    session: Session,
    tokens: [i64; 3],
}

struct Inner {
    manager: OnceLock<Manager>,
    sink: OnceLock<MediaSink>,
    subscription: Mutex<Option<Subscription>>,
    artwork_dir: PathBuf,
    /// (track identity, cached artwork path) so artwork is read once per track.
    artwork_cache: Mutex<Option<(String, Option<String>)>>,
    /// Serializes state reads; WinRT raises change events on arbitrary threads.
    read_lock: Mutex<()>,
    /// Bumped on every change notification so stale retries give up.
    generation: AtomicU64,
    /// App id of the session most recently seen playing; kept on screen after it pauses.
    sticky: Mutex<Option<String>>,
    /// Play/pause watchers on *every* session, so one that starts playing while another is
    /// shown is noticed (Windows doesn't always raise `CurrentSessionChanged` for that).
    watchers: Mutex<Vec<(Session, i64)>>,
}

pub struct WindowsMediaPort {
    inner: Arc<Inner>,
}

impl WindowsMediaPort {
    pub fn new(artwork_dir: PathBuf) -> Self {
        Self {
            inner: Arc::new(Inner {
                manager: OnceLock::new(),
                sink: OnceLock::new(),
                subscription: Mutex::new(None),
                artwork_dir,
                artwork_cache: Mutex::new(None),
                read_lock: Mutex::new(()),
                generation: AtomicU64::new(0),
                sticky: Mutex::new(None),
                watchers: Mutex::new(Vec::new()),
            }),
        }
    }
}

impl MediaPort for WindowsMediaPort {
    fn start(&self, sink: MediaSink) -> AppResult<()> {
        let _ = self.inner.sink.set(sink);
        let inner = self.inner.clone();
        // A dedicated MTA thread owns setup, so blocking WinRT calls never touch the UI thread.
        std::thread::Builder::new()
            .name("media-session".into())
            .spawn(move || {
                // SAFETY: initializes the WinRT apartment for this thread only.
                if let Err(error) = unsafe { RoInitialize(RO_INIT_MULTITHREADED) } {
                    tracing::debug!(error = %error.message(), "RoInitialize (already initialized?)");
                }
                if let Err(error) = inner.clone().connect() {
                    tracing::warn!(%error, "media sessions unavailable");
                    inner.emit(None);
                    return;
                }
                // Keep this thread (and its apartment) alive for the life of the app.
                loop {
                    std::thread::park();
                }
            })
            .map_err(AppError::Io)?;
        Ok(())
    }

    fn command(&self, command: MediaCommand) -> AppResult<()> {
        // Always act on the session the notch is showing, never on whatever Windows
        // currently considers "current" (they can differ, e.g. after a pause).
        let session = self
            .inner
            .bound_session()
            .ok_or_else(|| AppError::Platform("nothing to control".into()))?;
        let accepted = match command {
            MediaCommand::PlayPause => session.TryTogglePlayPauseAsync(),
            MediaCommand::Next => session.TrySkipNextAsync(),
            MediaCommand::Previous => session.TrySkipPreviousAsync(),
            MediaCommand::Seek { position_ms } => {
                session.TryChangePlaybackPositionAsync(ms_to_ticks(position_ms))
            }
        }
        .and_then(|op| op.get())
        .map_err(platform)?;
        if accepted {
            Ok(())
        } else {
            Err(AppError::Platform("the media app declined the request".into()))
        }
    }
}

impl Inner {
    fn connect(self: Arc<Self>) -> AppResult<()> {
        let manager = Manager::RequestAsync()
            .and_then(|op| op.get())
            .map_err(platform)?;
        let (current, added_removed) = (Arc::downgrade(&self), Arc::downgrade(&self));
        manager
            .CurrentSessionChanged(&TypedEventHandler::new(move |_, _| {
                if let Some(inner) = current.upgrade() {
                    inner.rebind();
                }
                Ok(())
            }))
            .map_err(platform)?;
        manager
            .SessionsChanged(&TypedEventHandler::new(move |_, _| {
                if let Some(inner) = added_removed.upgrade() {
                    inner.watch_all_sessions();
                    inner.rebind();
                }
                Ok(())
            }))
            .map_err(platform)?;
        let _ = self.manager.set(manager);
        self.watch_all_sessions();
        self.rebind();
        Ok(())
    }

    /// Re-attach a play/pause watcher to every live session (called when sessions come and go).
    fn watch_all_sessions(self: &Arc<Self>) {
        let mut watchers = self.watchers.lock().unwrap_or_else(|e| e.into_inner());
        for (session, token) in watchers.drain(..) {
            let _ = session.RemovePlaybackInfoChanged(token);
        }
        let Some(sessions) = self.manager.get().and_then(|m| m.GetSessions().ok()) else {
            return;
        };
        for session in sessions {
            let weak = Arc::downgrade(self);
            let token = session.PlaybackInfoChanged(&TypedEventHandler::new(move |_, _| {
                if let Some(inner) = weak.upgrade() {
                    inner.rebind();
                }
                Ok(())
            }));
            match token {
                Ok(token) => watchers.push((session, token)),
                Err(error) => tracing::debug!(error = %error.message(), "could not watch a media session"),
            }
        }
    }

    fn bound_session(&self) -> Option<Session> {
        let subscription = self.subscription.lock().unwrap_or_else(|e| e.into_inner());
        subscription.as_ref().map(|s| s.session.clone())
    }

    /// Pick the session to show (see [`choose_session`]) from all live sessions.
    fn select_session(&self) -> Option<Session> {
        let manager = self.manager.get()?;
        let sessions: Vec<Session> = manager.GetSessions().ok()?.into_iter().collect();
        let candidates: Vec<(String, bool)> = sessions
            .iter()
            .map(|s| {
                let id = s
                    .SourceAppUserModelId()
                    .map(|h| h.to_string_lossy())
                    .unwrap_or_default();
                let playing = s
                    .GetPlaybackInfo()
                    .and_then(|p| p.PlaybackStatus())
                    .is_ok_and(|status| status == WinStatus::Playing);
                (id, playing)
            })
            .collect();
        let bound = self
            .bound_session()
            .and_then(|s| s.SourceAppUserModelId().ok())
            .map(|h| h.to_string_lossy());
        let current = manager
            .GetCurrentSession()
            .and_then(|s| s.SourceAppUserModelId())
            .map(|h| h.to_string_lossy())
            .ok();
        let sticky = self.sticky.lock().unwrap_or_else(|e| e.into_inner()).clone();
        let index = choose_session(
            &candidates,
            bound.as_deref(),
            sticky.as_deref(),
            current.as_deref(),
        )?;
        if candidates[index].1 {
            *self.sticky.lock().unwrap_or_else(|e| e.into_inner()) = Some(candidates[index].0.clone());
        }
        sessions.into_iter().nth(index)
    }

    /// Re-select the session to show, move change subscriptions to it if it changed, and
    /// publish its state.
    fn rebind(self: &Arc<Self>) {
        let selected = self.select_session();
        let selected_id = selected
            .as_ref()
            .and_then(|s| s.SourceAppUserModelId().ok())
            .map(|h| h.to_string_lossy());

        let mut subscription = self.subscription.lock().unwrap_or_else(|e| e.into_inner());
        let bound_id = subscription
            .as_ref()
            .and_then(|s| s.session.SourceAppUserModelId().ok())
            .map(|h| h.to_string_lossy());
        if bound_id != selected_id || subscription.is_none() {
            if let Some(old) = subscription.take() {
                let _ = old.session.RemoveMediaPropertiesChanged(old.tokens[0]);
                let _ = old.session.RemovePlaybackInfoChanged(old.tokens[1]);
                let _ = old.session.RemoveTimelinePropertiesChanged(old.tokens[2]);
            }
            if let Some(session) = selected {
                match self.subscribe(&session) {
                    Ok(tokens) => *subscription = Some(Subscription { session, tokens }),
                    Err(error) => tracing::warn!(error = %error.message(), "could not watch media session"),
                }
            }
        }
        drop(subscription);
        self.refresh();
    }

    fn subscribe(self: &Arc<Self>, session: &Session) -> windows_core::Result<[i64; 3]> {
        let (a, b, c) = (Arc::downgrade(self), Arc::downgrade(self), Arc::downgrade(self));
        Ok([
            session.MediaPropertiesChanged(&TypedEventHandler::new(move |_, _| {
                if let Some(inner) = a.upgrade() {
                    inner.refresh();
                }
                Ok(())
            }))?,
            // Play/pause can change which session should be on screen, so re-select.
            session.PlaybackInfoChanged(&TypedEventHandler::new(move |_, _| {
                if let Some(inner) = b.upgrade() {
                    inner.rebind();
                }
                Ok(())
            }))?,
            session.TimelinePropertiesChanged(&TypedEventHandler::new(move |_, _| {
                if let Some(inner) = c.upgrade() {
                    inner.refresh();
                }
                Ok(())
            }))?,
        ])
    }

    fn refresh(self: &Arc<Self>) {
        let generation = self.generation.fetch_add(1, Ordering::SeqCst) + 1;
        self.refresh_attempt(generation, 0);
    }

    fn refresh_attempt(self: &Arc<Self>, generation: u64, attempt: u32) {
        let _guard = self.read_lock.lock().unwrap_or_else(|e| e.into_inner());
        if self.generation.load(Ordering::SeqCst) != generation {
            return; // a newer change is already being handled
        }
        let Some(session) = self.bound_session() else {
            self.emit(None);
            return;
        };
        match self.read_state(&session) {
            Ok(state) => self.emit(Some(state)),
            Err(error) if attempt < READ_RETRIES => {
                tracing::debug!(error = %error.message(), attempt, "media session not readable yet; retrying");
                let inner = self.clone();
                std::thread::spawn(move || {
                    std::thread::sleep(READ_RETRY_DELAY);
                    inner.refresh_attempt(generation, attempt + 1);
                });
            }
            Err(error) => {
                // Still unreadable (e.g. the session closed mid-read): report nothing playing.
                tracing::debug!(error = %error.message(), "could not read media session");
                self.emit(None);
            }
        }
    }

    fn emit(&self, state: Option<MediaState>) {
        if let Some(sink) = self.sink.get() {
            sink(state);
        }
    }

    fn read_state(&self, session: &Session) -> windows_core::Result<MediaState> {
        // Some apps publish playback state before (or without) track metadata; that read then
        // fails with "device not ready". Treat metadata as optional rather than dropping the session.
        let properties = match session.TryGetMediaPropertiesAsync().and_then(|op| op.get()) {
            Ok(properties) => Some(properties),
            Err(error) => {
                tracing::debug!(error = %error.message(), "media session has no readable metadata");
                None
            }
        };
        let playback = session.GetPlaybackInfo()?;
        let controls = playback.Controls()?;
        let timeline = session.GetTimelineProperties()?;

        let status = match playback.PlaybackStatus()? {
            WinStatus::Playing => PlaybackStatus::Playing,
            WinStatus::Paused => PlaybackStatus::Paused,
            WinStatus::Stopped | WinStatus::Closed => PlaybackStatus::Stopped,
            _ => PlaybackStatus::Other,
        };
        let text = |read: fn(&MediaProperties) -> windows_core::Result<windows_core::HSTRING>| {
            properties
                .as_ref()
                .and_then(|p| read(p).ok())
                .map(|h| h.to_string_lossy())
                .unwrap_or_default()
        };
        let title = text(MediaProperties::Title);
        let artist = text(MediaProperties::Artist);
        let album = text(MediaProperties::AlbumTitle);

        let duration = ticks_to_ms(timeline.EndTime()?.Duration - timeline.StartTime()?.Duration);
        let position = ticks_to_ms(timeline.Position()?.Duration - timeline.StartTime()?.Duration);
        let has_timeline = duration > 0;

        Ok(MediaState {
            source_app: display_name(&session.SourceAppUserModelId()?.to_string_lossy()),
            artwork_path: properties
                .as_ref()
                .and_then(|p| self.artwork_for(p, &format!("{title}\u{1f}{artist}\u{1f}{album}"))),
            title,
            artist,
            album,
            status,
            position_ms: has_timeline.then_some(position.min(duration)),
            duration_ms: has_timeline.then_some(duration),
            position_updated_at: filetime_to_utc(timeline.LastUpdatedTime()?.UniversalTime)
                .unwrap_or_else(Utc::now),
            // Some apps report 0 (or nothing) while playing; that would freeze the progress bar.
            rate: playback
                .PlaybackRate()
                .and_then(|r| r.Value())
                .ok()
                .filter(|rate| *rate > 0.0)
                .unwrap_or(1.0),
            capabilities: MediaCapabilities {
                play_pause: controls.IsPlayPauseToggleEnabled()?,
                next: controls.IsNextEnabled()?,
                previous: controls.IsPreviousEnabled()?,
                seek: controls.IsPlaybackPositionEnabled()? && has_timeline,
            },
            mock: false,
        })
    }

    /// Cached artwork path for the current track; reads the thumbnail only when the track changes.
    fn artwork_for(&self, properties: &MediaProperties, track: &str) -> Option<String> {
        let mut cache = self.artwork_cache.lock().unwrap_or_else(|e| e.into_inner());
        if let Some((cached_track, path)) = cache.as_ref() {
            if cached_track == track {
                return path.clone();
            }
        }
        let path = match read_thumbnail(properties)
            .and_then(|(bytes, ext)| store_artwork(&self.artwork_dir, &bytes, ext))
        {
            Ok(path) => path.map(|p| p.display().to_string()),
            Err(error) => {
                tracing::debug!(%error, "no artwork for current track");
                None
            }
        };
        // Some apps publish artwork a moment after the title; don't cache a miss forever.
        if path.is_some() {
            *cache = Some((track.to_owned(), path.clone()));
        }
        path
    }
}

fn read_thumbnail(properties: &MediaProperties) -> AppResult<(Vec<u8>, &'static str)> {
    // Sessions without artwork return a null reference, surfaced as an "error" with S_OK.
    let reference = properties
        .Thumbnail()
        .map_err(|_| AppError::InvalidInput("session has no artwork".into()))?;
    let stream = reference
        .OpenReadAsync()
        .and_then(|op| op.get())
        .map_err(platform)?;
    let size = stream.Size().map_err(platform)?;
    if size == 0 || size > MAX_ARTWORK_BYTES {
        return Err(AppError::InvalidInput(format!(
            "artwork size {size} out of range"
        )));
    }
    let content_type = stream
        .ContentType()
        .map(|t| t.to_string_lossy())
        .unwrap_or_default();
    let reader = DataReader::CreateDataReader(&stream).map_err(platform)?;
    reader
        .LoadAsync(size as u32)
        .and_then(|op| op.get())
        .map_err(platform)?;
    let mut bytes = vec![0u8; size as usize];
    reader.ReadBytes(&mut bytes).map_err(platform)?;
    let ext = extension_for(&content_type, &bytes);
    Ok((bytes, ext))
}

/// Choose which session to show from `(app id, is_playing)` candidates:
/// 1. something playing — preferring what is already shown, then the last one seen playing,
///    then Windows' current session;
/// 2. otherwise the last session seen playing, so a paused app stays put for resuming;
/// 3. otherwise what is already shown, then Windows' current session, then the first.
pub fn choose_session(
    candidates: &[(String, bool)],
    bound: Option<&str>,
    sticky: Option<&str>,
    current: Option<&str>,
) -> Option<usize> {
    let find = |id: Option<&str>, must_play: bool| {
        let id = id?;
        candidates
            .iter()
            .position(|(c, playing)| c == id && (!must_play || *playing))
    };
    find(bound, true)
        .or_else(|| find(sticky, true))
        .or_else(|| find(current, true))
        .or_else(|| candidates.iter().position(|(_, playing)| *playing))
        .or_else(|| find(sticky, false))
        .or_else(|| find(bound, false))
        .or_else(|| find(current, false))
        .or(if candidates.is_empty() { None } else { Some(0) })
}

/// File extension from the declared content type, falling back to magic bytes.
pub fn extension_for(content_type: &str, bytes: &[u8]) -> &'static str {
    match content_type {
        "image/png" => "png",
        "image/jpeg" | "image/jpg" => "jpg",
        "image/webp" => "webp",
        _ if bytes.starts_with(&[0x89, b'P', b'N', b'G']) => "png",
        _ if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) => "jpg",
        _ => "img",
    }
}

/// Write artwork under a content hash (idempotent) and prune old files.
pub fn store_artwork(dir: &Path, bytes: &[u8], ext: &str) -> AppResult<Option<PathBuf>> {
    std::fs::create_dir_all(dir)?;
    let mut hasher = DefaultHasher::new();
    bytes.hash(&mut hasher);
    let path = dir.join(format!("{:016x}.{ext}", hasher.finish()));
    if path.exists() {
        // Mark it recently used so pruning can't delete artwork that's on screen.
        let _ = std::fs::File::options()
            .append(true)
            .open(&path)
            .and_then(|f| f.set_modified(std::time::SystemTime::now()));
    } else {
        std::fs::write(&path, bytes)?;
        prune_artwork(dir, MAX_ARTWORK_FILES)?;
    }
    Ok(Some(path))
}

fn prune_artwork(dir: &Path, keep: usize) -> AppResult<()> {
    let mut files: Vec<_> = std::fs::read_dir(dir)?
        .filter_map(|e| e.ok())
        .filter_map(|e| Some((e.metadata().ok()?.modified().ok()?, e.path())))
        .collect();
    if files.len() <= keep {
        return Ok(());
    }
    files.sort_by_key(|(modified, _)| *modified);
    for (_, path) in files.iter().take(files.len() - keep) {
        let _ = std::fs::remove_file(path);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reused_artwork_survives_pruning() {
        let dir = std::env::temp_dir().join(format!("notch-artwork-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let on_screen = store_artwork(&dir, b"cover-0", "png").unwrap().unwrap();
        for i in 1..MAX_ARTWORK_FILES {
            store_artwork(&dir, format!("cover-{i}").as_bytes(), "png").unwrap();
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
        // Shown again, then one more new cover pushes the cache over its limit.
        assert_eq!(
            store_artwork(&dir, b"cover-0", "png").unwrap().unwrap(),
            on_screen
        );
        store_artwork(&dir, b"cover-new", "png").unwrap();
        assert!(on_screen.exists());
        assert_eq!(std::fs::read_dir(&dir).unwrap().count(), MAX_ARTWORK_FILES);
        let _ = std::fs::remove_dir_all(&dir);
    }

    fn c(list: &[(&str, bool)]) -> Vec<(String, bool)> {
        list.iter()
            .map(|(id, playing)| (id.to_string(), *playing))
            .collect()
    }

    #[test]
    fn prefers_a_playing_session() {
        let sessions = c(&[("netflix", false), ("edge", true)]);
        assert_eq!(
            choose_session(&sessions, Some("netflix"), None, Some("netflix")),
            Some(1)
        );
    }

    #[test]
    fn keeps_a_paused_session_that_was_playing_instead_of_jumping() {
        // Edge was playing and is now paused; Windows moved "current" to an old Netflix session.
        let sessions = c(&[("netflix", false), ("edge", false)]);
        assert_eq!(
            choose_session(&sessions, Some("edge"), Some("edge"), Some("netflix")),
            Some(1)
        );
    }

    #[test]
    fn keeps_showing_the_bound_session_when_several_play() {
        let sessions = c(&[("spotify", true), ("edge", true)]);
        assert_eq!(
            choose_session(&sessions, Some("edge"), Some("spotify"), Some("spotify")),
            Some(1)
        );
    }

    #[test]
    fn falls_back_to_windows_current_then_first() {
        let sessions = c(&[("a", false), ("b", false)]);
        assert_eq!(choose_session(&sessions, None, None, Some("b")), Some(1));
        assert_eq!(choose_session(&sessions, None, None, None), Some(0));
        assert_eq!(choose_session(&[], None, None, None), None);
    }

    #[test]
    fn picks_extensions_from_type_or_magic_bytes() {
        assert_eq!(extension_for("image/png", &[]), "png");
        assert_eq!(extension_for("", &[0xFF, 0xD8, 0xFF, 0xE0]), "jpg");
        assert_eq!(extension_for("application/octet-stream", b"\x89PNG\r\n"), "png");
        assert_eq!(extension_for("", b"GIF89a"), "img");
    }

    #[test]
    fn stores_artwork_idempotently_and_prunes() {
        let dir = std::env::temp_dir().join(format!("notch-artwork-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);

        let a = store_artwork(&dir, b"one", "png").unwrap().unwrap();
        let again = store_artwork(&dir, b"one", "png").unwrap().unwrap();
        assert_eq!(a, again);

        for i in 0..(MAX_ARTWORK_FILES + 5) {
            store_artwork(&dir, format!("img-{i}").as_bytes(), "png").unwrap();
        }
        assert!(std::fs::read_dir(&dir).unwrap().count() <= MAX_ARTWORK_FILES);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
