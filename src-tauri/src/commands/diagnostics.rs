use std::path::{Path, PathBuf};

use tauri::{AppHandle, Manager, State};
use tauri_plugin_opener::OpenerExt;

use crate::{
    error::{AppError, AppResult},
    state::AppState,
};

fn open(app: &AppHandle, path: &Path) -> AppResult<()> {
    app.opener()
        .open_path(path.display().to_string(), None::<&str>)
        .map_err(|e| AppError::Platform(e.to_string()))
}

/// Open the local log folder in Explorer. Diagnostics stay on this PC unless the user
/// chooses to share them (architecture.md §12).
#[tauri::command]
pub fn diagnostics_open_log_dir(app: AppHandle, state: State<'_, AppState>) -> AppResult<()> {
    open(&app, &state.paths.log_dir)
}

/// Copy the log files into a fresh folder under Downloads and reveal it, ready to attach
/// to a bug report. Returns the folder path.
#[tauri::command]
pub fn diagnostics_export_logs(app: AppHandle, state: State<'_, AppState>) -> AppResult<String> {
    let downloads = app.path().download_dir()?;
    let stamp = chrono::Local::now().format("%Y%m%d-%H%M%S");
    let target: PathBuf = downloads.join(format!("notch-desktop-logs-{stamp}"));
    std::fs::create_dir_all(&target)?;

    let mut copied = 0;
    for entry in std::fs::read_dir(&state.paths.log_dir)? {
        let path = entry?.path();
        let is_log = path
            .file_name()
            .and_then(|n| n.to_str())
            .is_some_and(|n| n.starts_with("notch.log"));
        if is_log {
            if let Some(name) = path.file_name() {
                std::fs::copy(&path, target.join(name))?;
                copied += 1;
            }
        }
    }
    tracing::info!(copied, "exported diagnostic logs");
    open(&app, &target)?;
    Ok(target.display().to_string())
}
