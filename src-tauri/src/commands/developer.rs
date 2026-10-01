use tauri::{AppHandle, Manager, State};

use crate::{
    developer::{
        runner::{Language, RunResult},
        service::DeveloperService,
        widgets::WidgetOutput,
        CodingStats, DevServer,
    },
    error::{AppError, AppResult},
    git::service::GitService,
};

async fn blocking<T: Send + 'static>(f: impl FnOnce() -> AppResult<T> + Send + 'static) -> AppResult<T> {
    tauri::async_runtime::spawn_blocking(f)
        .await
        .map_err(|e| AppError::Platform(e.to_string()))?
}

/// Local dev servers (looked up now).
#[tauri::command]
pub async fn dev_servers(dev: State<'_, DeveloperService>) -> AppResult<Vec<DevServer>> {
    let dev = dev.inner().clone();
    blocking(move || Ok(dev.scan())).await
}

#[tauri::command]
pub async fn dev_stop_server(pid: u32, dev: State<'_, DeveloperService>) -> AppResult<()> {
    let dev = dev.inner().clone();
    blocking(move || dev.stop(pid)).await
}

#[tauri::command]
pub async fn coding_stats(dev: State<'_, DeveloperService>) -> AppResult<CodingStats> {
    let dev = dev.inner().clone();
    blocking(move || dev.stats()).await
}

/// Run a few lines of JavaScript or Python.
#[tauri::command]
pub async fn dev_run(
    language: Language,
    code: String,
    dev: State<'_, DeveloperService>,
) -> AppResult<RunResult> {
    let dev = dev.inner().clone();
    blocking(move || dev.run(language, &code)).await
}

#[tauri::command]
pub async fn dev_widgets(dev: State<'_, DeveloperService>) -> AppResult<Vec<WidgetOutput>> {
    Ok(dev.inner().clone().widgets().await)
}

/// A commit message for the current changes, written by Claude Code.
#[tauri::command]
pub async fn git_suggest_message(
    include_untracked: bool,
    app: AppHandle,
    git: State<'_, GitService>,
) -> AppResult<String> {
    let workdir = app
        .path()
        .app_cache_dir()
        .map_err(|e| AppError::Platform(e.to_string()))?
        .join("ask");
    git.inner()
        .clone()
        .suggest_message(include_untracked, workdir)
        .await
}
