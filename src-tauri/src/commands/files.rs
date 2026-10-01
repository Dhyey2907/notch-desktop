use tauri::{State, WebviewWindow};

use crate::{
    error::{AppError, AppResult},
    files::{convert::ImageTarget, service::FilesService, DownloadProgress, RecentFile, RecorderState},
};

async fn blocking<T: Send + 'static>(f: impl FnOnce() -> AppResult<T> + Send + 'static) -> AppResult<T> {
    tauri::async_runtime::spawn_blocking(f)
        .await
        .map_err(|e| AppError::Platform(e.to_string()))?
}

#[tauri::command]
pub fn files_downloads(files: State<'_, FilesService>) -> Vec<DownloadProgress> {
    files.downloads()
}

/// Files opened recently in any app (from Windows' Recent list).
#[tauri::command]
pub async fn files_recent(files: State<'_, FilesService>) -> AppResult<Vec<RecentFile>> {
    let files = files.inner().clone();
    blocking(move || Ok(files.recent())).await
}

/// Save a copy of an image as PNG, JPG or WebP; returns the new file's path.
#[tauri::command]
pub async fn files_convert_image(
    path: String,
    to: ImageTarget,
    files: State<'_, FilesService>,
) -> AppResult<String> {
    let files = files.inner().clone();
    blocking(move || files.convert(&path, to)).await
}

/// Save a smaller copy of an image for sharing.
#[tauri::command]
pub async fn files_shrink_image(path: String, files: State<'_, FilesService>) -> AppResult<String> {
    let files = files.inner().clone();
    blocking(move || files.shrink(&path)).await
}

/// Join PDFs in this order into one.
#[tauri::command]
pub async fn files_merge_pdfs(paths: Vec<String>, files: State<'_, FilesService>) -> AppResult<String> {
    let files = files.inner().clone();
    blocking(move || files.merge(&paths)).await
}

#[tauri::command]
pub fn files_open(path: String, files: State<'_, FilesService>) -> AppResult<()> {
    files.open(&path)
}

#[tauri::command]
pub fn files_reveal(path: String, files: State<'_, FilesService>) -> AppResult<()> {
    files.reveal(&path)
}

#[tauri::command]
pub async fn files_drag(
    paths: Vec<String>,
    window: WebviewWindow,
    files: State<'_, FilesService>,
) -> AppResult<bool> {
    files.drag(window, paths).await
}

#[tauri::command]
pub fn recorder_state(files: State<'_, FilesService>) -> RecorderState {
    files.recorder()
}

/// Start recording what the PC plays.
#[tauri::command]
pub fn recorder_start(files: State<'_, FilesService>) -> AppResult<RecorderState> {
    files.record_start()
}

/// Stop and save; the file goes on the shelf.
#[tauri::command]
pub async fn recorder_stop(files: State<'_, FilesService>) -> AppResult<Option<String>> {
    let files = files.inner().clone();
    blocking(move || files.record_stop()).await
}
