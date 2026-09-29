use tauri::State;

use crate::{
    error::AppResult,
    git::{service::GitService, GitOperation, GitSnapshot, PullRequests, RepoInfo},
};

#[tauri::command]
pub fn git_get_snapshot(git: State<'_, GitService>) -> GitSnapshot {
    git.snapshot()
}

#[tauri::command]
pub async fn git_refresh(git: State<'_, GitService>) -> AppResult<()> {
    git.refresh().await;
    Ok(())
}

/// Folder picker; `None` if the user cancelled.
#[tauri::command]
pub async fn git_pick_repo(git: State<'_, GitService>) -> AppResult<Option<RepoInfo>> {
    git.pick_repo().await
}

#[tauri::command]
pub async fn git_select_repo(path: String, git: State<'_, GitService>) -> AppResult<RepoInfo> {
    git.select_repo(path).await
}

#[tauri::command]
pub fn git_forget_repo(path: String, git: State<'_, GitService>) -> AppResult<()> {
    git.forget_repo(&path)
}

/// Commit, push, or pull. Only invoked after the user confirms in the UI.
#[tauri::command]
pub async fn git_run(
    operation: GitOperation,
    message: Option<String>,
    include_untracked: Option<bool>,
    git: State<'_, GitService>,
) -> AppResult<()> {
    git.run(operation, message, include_untracked.unwrap_or(false))
        .await
}

#[tauri::command]
pub async fn git_pull_requests(git: State<'_, GitService>) -> AppResult<PullRequests> {
    Ok(git.pull_requests().await)
}

#[tauri::command]
pub fn git_open_folder(git: State<'_, GitService>) -> AppResult<()> {
    git.open_folder()
}
