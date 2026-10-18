use tauri::State;

use crate::{
    error::AppResult,
    meeting::{service::MeetingService, MeetingState},
};

#[tauri::command]
pub fn meeting_state(meeting: State<'_, MeetingService>) -> MeetingState {
    meeting.state()
}

/// Download Whisper and its speech model (about 150 MB); progress comes as MeetingChanged.
#[tauri::command]
pub async fn meeting_install(meeting: State<'_, MeetingService>) -> AppResult<()> {
    let meeting = meeting.inner().clone();
    meeting.install().await
}

/// Start recording the meeting (your microphone and what the PC plays).
#[tauri::command]
pub fn meeting_start(meeting: State<'_, MeetingService>) -> AppResult<MeetingState> {
    meeting.start()
}

/// Stop; notes are made in the background (MeetingDone when ready).
#[tauri::command]
pub fn meeting_stop(meeting: State<'_, MeetingService>) -> AppResult<()> {
    meeting.stop()
}

/// Stop and throw the recording away.
#[tauri::command]
pub fn meeting_cancel(meeting: State<'_, MeetingService>) {
    meeting.cancel();
}
