//! Development-only media adapter (`NOTCH_MOCK=media`). Its state is flagged `mock: true`,
//! and the UI badges it, so sample data can never pass for real playback (plan.md Rule 5).

use std::sync::Mutex;

use chrono::Utc;

use super::{MediaCapabilities, MediaCommand, MediaPort, MediaSink, MediaState, PlaybackStatus};
use crate::error::AppResult;

const TRACKS: [(&str, &str, &str); 3] = [
    ("Sample Track One", "Mock Artist", "Mock Album"),
    ("Sample Track Two", "Mock Artist", "Mock Album"),
    ("Sample Track Three", "Another Mock", "Mock Album"),
];
const DURATION_MS: u64 = 200_000;

#[derive(Default)]
pub struct MockMediaPort {
    sink: Mutex<Option<MediaSink>>,
    state: Mutex<(usize, bool, u64)>, // (track index, playing, position)
}

impl MockMediaPort {
    fn snapshot(&self) -> MediaState {
        let (index, playing, position) = *self.state.lock().unwrap_or_else(|e| e.into_inner());
        let (title, artist, album) = TRACKS[index];
        MediaState {
            source_app: "Mock player".into(),
            title: title.into(),
            artist: artist.into(),
            album: album.into(),
            artwork_path: None,
            status: if playing {
                PlaybackStatus::Playing
            } else {
                PlaybackStatus::Paused
            },
            position_ms: Some(position),
            duration_ms: Some(DURATION_MS),
            position_updated_at: Utc::now(),
            rate: 1.0,
            capabilities: MediaCapabilities {
                play_pause: true,
                next: true,
                previous: true,
                seek: true,
            },
            mock: true,
        }
    }

    fn publish(&self) {
        let sink = self.sink.lock().unwrap_or_else(|e| e.into_inner()).clone();
        if let Some(sink) = sink {
            sink(Some(self.snapshot()));
        }
    }
}

impl MediaPort for MockMediaPort {
    fn start(&self, sink: MediaSink) -> AppResult<()> {
        *self.sink.lock().unwrap_or_else(|e| e.into_inner()) = Some(sink);
        *self.state.lock().unwrap_or_else(|e| e.into_inner()) = (0, true, 30_000);
        self.publish();
        Ok(())
    }

    fn command(&self, command: MediaCommand) -> AppResult<()> {
        {
            let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
            match command {
                MediaCommand::PlayPause => state.1 = !state.1,
                MediaCommand::Next => *state = ((state.0 + 1) % TRACKS.len(), true, 0),
                MediaCommand::Previous => *state = ((state.0 + TRACKS.len() - 1) % TRACKS.len(), true, 0),
                MediaCommand::Seek { position_ms } => state.2 = position_ms.min(DURATION_MS),
            }
        }
        self.publish();
        Ok(())
    }
}
