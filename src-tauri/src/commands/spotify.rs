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

/// The song playing on Spotify, whether it's liked, and what's up next.
#[tauri::command]
pub async fn spotify_now(spotify: State<'_, SpotifyService>) -> AppResult<crate::spotify::SpotifyNow> {
    let spotify = spotify.inner().clone();
    spotify.now().await
}

/// Add the song to Liked Songs (or take it out).
#[tauri::command]
pub async fn spotify_like(
    track_id: String,
    liked: bool,
    spotify: State<'_, SpotifyService>,
) -> AppResult<()> {
    let spotify = spotify.inner().clone();
    spotify.set_liked(&track_id, liked).await
}
