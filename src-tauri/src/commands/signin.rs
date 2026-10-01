//! One-click "Continue with …" sign-ins using Notch's built-in OAuth clients.

use tauri::State;

use crate::{
    calendar::service::CalendarService,
    error::{AppError, AppResult},
    git::{service::GitService, RepoInfo},
    github::{service::GithubService, DeviceCode, GithubRepo, GithubRepoOverview, GithubStatus},
    oauth_clients::{self, SignInOptions},
    spotify::service::SpotifyService,
};

fn not_configured(provider: &str) -> AppError {
    AppError::InvalidInput(format!("This build has no {provider} sign-in configured"))
}

/// Which "Continue with …" buttons this build can offer.
#[tauri::command]
pub fn sign_in_options() -> SignInOptions {
    oauth_clients::options()
}

/// "Continue with Google": the regular Calendar sign-in with the built-in client.
#[tauri::command]
pub async fn calendar_connect_builtin(calendar: State<'_, CalendarService>) -> AppResult<()> {
    let (id, secret) = oauth_clients::google().ok_or_else(|| not_configured("Google"))?;
    calendar.connect(id.to_owned(), secret.to_owned()).await
}

/// "Continue with Spotify": the regular Spotify sign-in with the built-in client.
#[tauri::command]
pub async fn spotify_connect_builtin(spotify: State<'_, SpotifyService>) -> AppResult<()> {
    let id = oauth_clients::spotify().ok_or_else(|| not_configured("Spotify"))?;
    spotify.connect(id.to_owned()).await
}

#[tauri::command]
pub fn github_status(github: State<'_, GithubService>) -> GithubStatus {
    github.status()
}

/// "Continue with GitHub": returns the code to enter on github.com (opened in the browser).
#[tauri::command]
pub async fn github_sign_in(github: State<'_, GithubService>) -> AppResult<DeviceCode> {
    github.sign_in().await
}

#[tauri::command]
pub fn github_cancel_sign_in(github: State<'_, GithubService>) {
    github.cancel_sign_in()
}

#[tauri::command]
pub fn github_sign_out(github: State<'_, GithubService>) -> AppResult<()> {
    github.sign_out()
}

/// Your GitHub repositories, most recently pushed first.
#[tauri::command]
pub async fn github_repositories(github: State<'_, GithubService>) -> AppResult<Vec<GithubRepo>> {
    github.repositories().await
}

/// Latest commits and open pull requests of a GitHub repository (`owner/name`).
#[tauri::command]
pub async fn github_repo_overview(
    name_with_owner: String,
    github: State<'_, GithubService>,
) -> AppResult<GithubRepoOverview> {
    github.overview(&name_with_owner).await
}

/// Show a GitHub repository in the Git module (its local clone, if you have one added).
#[tauri::command]
pub async fn github_select_repo(
    name_with_owner: String,
    git: State<'_, GitService>,
) -> AppResult<Option<RepoInfo>> {
    git.select_github_repo(name_with_owner).await
}

/// Unread GitHub notifications, newest first.
#[tauri::command]
pub fn github_notifications(
    github: State<'_, GithubService>,
) -> Vec<crate::github::notifications::GithubNotification> {
    github.notifications()
}

/// Mark a GitHub notification read (on GitHub too).
#[tauri::command]
pub async fn github_mark_read(id: String, github: State<'_, GithubService>) -> AppResult<()> {
    github.mark_read(&id).await
}
