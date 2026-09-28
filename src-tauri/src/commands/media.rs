use tauri::State;

use crate::{
    error::AppResult,
    media::{MediaCommand, MediaService, MediaState},
};

/// Latest known media state, for windows that start listening after the first event.
#[tauri::command]
pub fn media_get_state(media: State<'_, MediaService>) -> Option<MediaState> {
    media.latest()
}

/// Forward a playback control to the active session. `async` so the blocking WinRT call
/// runs off the main thread.
#[tauri::command]
pub async fn media_command(command: MediaCommand, media: State<'_, MediaService>) -> AppResult<()> {
    media.command(command)
}
