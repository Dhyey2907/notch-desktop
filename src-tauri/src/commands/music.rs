use tauri::State;

use crate::music::{service::MusicService, Lyrics, Palette};

/// Synced lyrics for the song playing now, if found.
#[tauri::command]
pub fn lyrics_current(music: State<'_, MusicService>) -> Option<Lyrics> {
    music.lyrics()
}

/// Two colors from a cached album-art file, for tinting the notch.
#[tauri::command]
pub async fn artwork_palette(path: String, music: State<'_, MusicService>) -> Result<Option<Palette>, ()> {
    let music = music.inner().clone();
    Ok(tauri::async_runtime::spawn_blocking(move || music.palette(&path))
        .await
        .ok()
        .flatten())
}
