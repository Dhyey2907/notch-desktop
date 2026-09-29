use tauri::State;

use crate::{
    error::AppResult,
    spotify::{service::SpotifyService, PlayOutcome, SpotifySnapshot},
};

#[tauri::command]
pub fn spotify_get_snapshot(spotify: State<'_, SpotifyService>) -> SpotifySnapshot {
    spotify.snapshot()
}

/// Opens Spotify sign-in in the browser and waits for it to finish.
#[tauri::command]
pub async fn spotify_connect(client_id: String, spotify: State<'_, SpotifyService>) -> AppResult<()> {
    spotify.connect(client_id).await
}

#[tauri::command]
pub async fn spotify_disconnect(spotify: State<'_, SpotifyService>) -> AppResult<()> {
    spotify.disconnect().await
}

#[tauri::command]
pub async fn spotify_refresh_playlists(spotify: State<'_, SpotifyService>) -> AppResult<()> {
    spotify.refresh_playlists().await
}

#[tauri::command]
pub async fn spotify_play_playlist(
    playlist_id: String,
    spotify: State<'_, SpotifyService>,
) -> AppResult<PlayOutcome> {
    spotify.play(&playlist_id).await
}

#[tauri::command]
pub fn spotify_set_pinned(playlist_ids: Vec<String>, spotify: State<'_, SpotifyService>) -> AppResult<()> {
    spotify.set_pinned(playlist_ids)
}
