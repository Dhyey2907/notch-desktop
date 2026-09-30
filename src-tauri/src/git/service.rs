//! Active repository, file watching, and Git operations.
//!
//! The watcher (not a timer) drives refreshes: changes under the repo are debounced and then
//! trigger one `git status` + `git log` (plan.md Rule 7). Write operations are serialized and
//! always come from an explicit, confirmed user action in the UI.

use std::{
    path::{Path, PathBuf},
    sync::{mpsc, Arc, Mutex},
    time::Duration,
};

use notify::{RecursiveMode, Watcher};
use tauri::{AppHandle, Manager};
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_opener::OpenerExt;

use super::{
    cli, repo_info, BranchStatus, Commit, DayCount, GitOperation, GitSnapshot, GitStatus, PullRequests,
    RepoInfo,
};
use crate::{
    commands::settings::persist_and_broadcast,
    error::{AppError, AppResult},
    events::{AppEvent, EventEnvelope},
    settings::Settings,
    state::AppState,
};

/// Days of commit history in the activity heatmap (26 weeks).
const ACTIVITY_DAYS: u32 = 182;
const RECENT_COMMITS: u32 = 5;
pub const MAX_RECENT_REPOS: usize = 8;
/// Quiet period after the last file change before refreshing.
const DEBOUNCE: Duration = Duration::from_millis(800);

#[derive(Default)]
struct State {
    repo: Option<PathBuf>,
    branch: BranchStatus,
    commits: Vec<Commit>,
    activity: Vec<DayCount>,
    error: Option<String>,
    running: Option<GitOperation>,
}

struct Inner {
    app: AppHandle,
    mock: bool,
    git_available: bool,
    state: Mutex<State>,
    /// Serializes write operations (commit/push/pull) per app.
    op_lock: tokio::sync::Mutex<()>,
    watcher: Mutex<Option<notify::RecommendedWatcher>>,
}

#[derive(Clone)]
pub struct GitService {
    inner: Arc<Inner>,
}

fn settings_of(app: &AppHandle) -> Settings {
    let state = app.state::<AppState>();
    let settings = state.settings.lock().unwrap_or_else(|e| e.into_inner()).clone();
    settings
}

fn update_settings(app: &AppHandle, change: impl FnOnce(&mut Settings)) -> AppResult<()> {
    let state = app.state::<AppState>();
    let mut next = settings_of(app);
    change(&mut next);
    persist_and_broadcast(&state, next).map(|_| ())
}

/// Paths whose changes can't affect `git status` or history, e.g. build output.
fn is_noise(path: &Path) -> bool {
    let text = path.to_string_lossy().replace('\\', "/");
    ["/node_modules/", "/target/", "/dist/", "/.git/objects/"]
        .iter()
        .any(|segment| text.contains(segment))
        || text.ends_with(".lock")
}

impl GitService {
    pub fn new(app: AppHandle, mock: bool) -> Self {
        let git_available = mock || cli::git_available();
        let repo = if mock {
            Some(PathBuf::from("C:/mock/notch-desktop"))
        } else {
            settings_of(&app)
                .git
                .active
                .map(PathBuf::from)
                .filter(|p| p.exists())
        };
        Self {
            inner: Arc::new(Inner {
                app,
                mock,
                git_available,
                state: Mutex::new(State {
                    repo,
                    ..Default::default()
                }),
                op_lock: tokio::sync::Mutex::new(()),
                watcher: Mutex::new(None),
            }),
        }
    }

    pub fn start(&self) {
        let inner = self.inner.clone();
        tauri::async_runtime::spawn_blocking(move || {
            inner.watch();
            inner.refresh();
        });
    }

    pub fn snapshot(&self) -> GitSnapshot {
        self.inner.snapshot()
    }

    pub async fn refresh(&self) {
        let inner = self.inner.clone();
        let _ = tauri::async_runtime::spawn_blocking(move || inner.refresh()).await;
    }

    /// Show a folder picker and switch to the chosen repository.
    pub async fn pick_repo(&self) -> AppResult<Option<RepoInfo>> {
        let app = self.inner.app.clone();
        let picked = tauri::async_runtime::spawn_blocking(move || {
            app.dialog()
                .file()
                .set_title("Choose a Git repository")
                .blocking_pick_folder()
        })
        .await
        .map_err(|e| AppError::Platform(e.to_string()))?;
        let Some(folder) = picked else { return Ok(None) };
        let path = folder
            .into_path()
            .map_err(|e| AppError::InvalidInput(e.to_string()))?;
        self.select_repo(path.display().to_string()).await.map(Some)
    }

    pub async fn select_repo(&self, path: String) -> AppResult<RepoInfo> {
        if path.len() > 400 {
            return Err(AppError::InvalidInput("path is too long".into()));
        }
        let top = if self.inner.mock {
            PathBuf::from(&path)
        } else {
            let requested = PathBuf::from(&path);
            tauri::async_runtime::spawn_blocking(move || cli::toplevel(&requested))
                .await
                .map_err(|e| AppError::Platform(e.to_string()))??
        };
        let top_str = top.display().to_string();
        update_settings(&self.inner.app, |s| {
            s.git.repos.retain(|r| !r.eq_ignore_ascii_case(&top_str));
            s.git.repos.insert(0, top_str.clone());
            s.git.repos.truncate(MAX_RECENT_REPOS);
            s.git.active = Some(top_str.clone());
        })?;
        {
            let mut state = self.inner.lock();
            *state = State {
                repo: Some(top.clone()),
                ..Default::default()
            };
        }
        self.start();
        Ok(repo_info(&top))
    }

    /// Show a GitHub repository. If one of your local repositories is a clone of it, that
    /// one opens (with commit/push/pull); otherwise the Git module shows it straight from
    /// GitHub. Returns the local repository used, if any.
    pub async fn select_github_repo(&self, name_with_owner: String) -> AppResult<Option<RepoInfo>> {
        if !crate::github::valid_name_with_owner(&name_with_owner) {
            return Err(AppError::InvalidInput(
                "that isn't a GitHub repository name".into(),
            ));
        }
        let wanted = name_with_owner.to_ascii_lowercase();
        let known = settings_of(&self.inner.app).git.repos;
        let local = tauri::async_runtime::spawn_blocking(move || {
            known.into_iter().find(|path| {
                cli::origin_url(std::path::Path::new(path))
                    .and_then(|url| crate::github::parse_remote(&url))
                    .is_some_and(|(owner, name)| format!("{owner}/{name}").to_ascii_lowercase() == wanted)
            })
        })
        .await
        .unwrap_or(None);
        let chosen = name_with_owner.clone();
        update_settings(&self.inner.app, |s| s.git.github_repo = Some(chosen))?;
        if let Some(path) = local {
            return self.select_repo(path).await.map(Some);
        }
        update_settings(&self.inner.app, |s| s.git.active = None)?;
        *self.inner.lock() = State::default();
        *self.inner.watcher.lock().unwrap_or_else(|e| e.into_inner()) = None;
        self.inner.publish();
        Ok(None)
    }

    pub fn forget_repo(&self, path: &str) -> AppResult<()> {
        let was_active = settings_of(&self.inner.app).git.active.as_deref() == Some(path);
        update_settings(&self.inner.app, |s| {
            s.git.repos.retain(|r| r != path);
            if was_active {
                s.git.active = None;
            }
        })?;
        if was_active {
            *self.inner.lock() = State::default();
            *self.inner.watcher.lock().unwrap_or_else(|e| e.into_inner()) = None;
        }
        self.inner.publish();
        Ok(())
    }

    /// Run a write operation. The UI only calls this after the user confirmed it.
    pub async fn run(
        &self,
        op: GitOperation,
        message: Option<String>,
        include_untracked: bool,
    ) -> AppResult<()> {
        let repo = self
            .inner
            .lock()
            .repo
            .clone()
            .ok_or_else(|| AppError::InvalidInput("choose a repository first".into()))?;
        let message = match op {
            GitOperation::Commit => {
                let m = message.unwrap_or_default().trim().to_owned();
                if m.is_empty() {
                    return Err(AppError::InvalidInput("write a commit message first".into()));
                }
                if m.len() > 2000 {
                    return Err(AppError::InvalidInput("commit message is too long".into()));
                }
                m
            }
            _ => String::new(),
        };
        let Ok(_guard) = self.inner.op_lock.try_lock() else {
            return Err(AppError::InvalidInput(
                "another Git operation is still running".into(),
            ));
        };
        let has_upstream = self.inner.lock().branch.upstream.is_some();
        self.inner.lock().running = Some(op);
        self.inner.publish();
        tracing::info!(?op, "git operation started");

        let mock = self.inner.mock;
        let result = tauri::async_runtime::spawn_blocking(move || {
            if mock {
                std::thread::sleep(Duration::from_millis(700));
                return Ok(());
            }
            match op {
                GitOperation::Commit => cli::commit(&repo, &message, include_untracked),
                GitOperation::Push => cli::push(&repo, has_upstream),
                GitOperation::Pull => cli::pull(&repo),
            }
        })
        .await
        .map_err(|e| AppError::Platform(e.to_string()))?;

        self.inner.lock().running = None;
        match &result {
            Ok(()) => tracing::info!(?op, "git operation finished"),
            // Only the kind is logged; git's message may name files or remotes.
            Err(error) => tracing::info!(?op, kind = error.kind(), "git operation failed"),
        }
        self.refresh().await;
        result
    }

    pub async fn pull_requests(&self) -> PullRequests {
        if self.inner.mock {
            return mock_pull_requests();
        }
        let Some(repo) = self.inner.lock().repo.clone() else {
            return PullRequests::Unavailable {
                reason: super::PrUnavailable::NotGithub,
            };
        };
        // Signed in with "Continue with GitHub": ask GitHub directly (no gh CLI needed).
        let github = self.inner.app.state::<crate::github::service::GithubService>();
        if github.signed_in() {
            let remote = {
                let repo = repo.clone();
                tauri::async_runtime::spawn_blocking(move || cli::origin_url(&repo))
                    .await
                    .ok()
                    .flatten()
            };
            return match remote.as_deref().and_then(crate::github::parse_remote) {
                Some((owner, name)) => github.pull_requests(&owner, &name).await,
                None => PullRequests::Unavailable {
                    reason: super::PrUnavailable::NotGithub,
                },
            };
        }
        tauri::async_runtime::spawn_blocking(move || cli::pull_requests(&repo))
            .await
            .unwrap_or(PullRequests::Unavailable {
                reason: super::PrUnavailable::Failed,
            })
    }

    pub fn open_folder(&self) -> AppResult<()> {
        let repo = self
            .inner
            .lock()
            .repo
            .clone()
            .ok_or_else(|| AppError::InvalidInput("choose a repository first".into()))?;
        self.inner
            .app
            .opener()
            .open_path(repo.display().to_string(), None::<&str>)
            .map_err(|e| AppError::Platform(e.to_string()))
    }
}

impl Inner {
    fn lock(&self) -> std::sync::MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn snapshot(&self) -> GitSnapshot {
        let recent_repos = if self.mock {
            vec![repo_info(Path::new("C:/mock/notch-desktop"))]
        } else {
            settings_of(&self.app)
                .git
                .repos
                .iter()
                .map(|r| repo_info(Path::new(r)))
                .collect()
        };
        let state = self.lock();
        let status = if !self.git_available {
            GitStatus::GitMissing
        } else if state.repo.is_none() {
            GitStatus::NoRepo
        } else if let Some(message) = &state.error {
            GitStatus::Error {
                message: message.clone(),
            }
        } else {
            GitStatus::Ready
        };
        GitSnapshot {
            status,
            repo: state.repo.as_deref().map(|p| {
                let mut info = repo_info(p);
                if self.mock {
                    info.name = "notch-desktop (MOCK)".into();
                }
                info
            }),
            branch: state.branch.clone(),
            commits: state.commits.clone(),
            activity: state.activity.clone(),
            recent_repos,
            running: state.running,
            mock: self.mock,
        }
    }

    fn publish(&self) {
        let envelope =
            EventEnvelope::new("git", AppEvent::GitChanged(self.snapshot())).with_dedup_key("git-snapshot");
        self.app.state::<AppState>().bus.publish(envelope);
    }

    /// Blocking: read status, recent commits, and activity for the active repo.
    fn refresh(&self) {
        let Some(repo) = self.lock().repo.clone() else {
            self.publish();
            return;
        };
        let read = if self.mock {
            Ok(mock_repo())
        } else {
            cli::status(&repo).and_then(|branch| {
                Ok((
                    branch,
                    cli::log(&repo, RECENT_COMMITS)?,
                    cli::commit_activity(&repo, ACTIVITY_DAYS)?,
                ))
            })
        };
        {
            let mut state = self.lock();
            if state.repo.as_deref() != Some(repo.as_path()) {
                return; // the user switched repos while we were reading
            }
            match read {
                Ok((branch, commits, activity)) => {
                    state.branch = branch;
                    state.commits = commits;
                    state.activity = activity;
                    state.error = None;
                }
                Err(error) => {
                    tracing::info!(kind = error.kind(), "could not read repository");
                    state.error = Some("This folder can't be read as a Git repository anymore.".into());
                }
            }
        }
        self.publish();
    }

    /// Watch the repository and refresh after changes settle.
    fn watch(self: &Arc<Self>) {
        *self.watcher.lock().unwrap_or_else(|e| e.into_inner()) = None;
        let Some(repo) = self.lock().repo.clone() else {
            return;
        };
        if self.mock {
            return;
        }
        let (tx, rx) = mpsc::channel::<()>();
        let watcher = notify::recommended_watcher(move |result: notify::Result<notify::Event>| {
            if let Ok(event) = result {
                if event.paths.iter().any(|p| !is_noise(p)) {
                    let _ = tx.send(());
                }
            }
        });
        let mut watcher = match watcher {
            Ok(w) => w,
            Err(error) => {
                tracing::warn!(%error, "could not create file watcher; Git status won't auto-refresh");
                return;
            }
        };
        if let Err(error) = watcher.watch(&repo, RecursiveMode::Recursive) {
            tracing::warn!(%error, "could not watch repository");
            return;
        }
        *self.watcher.lock().unwrap_or_else(|e| e.into_inner()) = Some(watcher);

        let weak = Arc::downgrade(self);
        std::thread::Builder::new()
            .name("git-watch".into())
            .spawn(move || {
                // Ends when the watcher (and its sender) is dropped on repo switch.
                while rx.recv().is_ok() {
                    // Wait for a quiet period so a burst of saves or a checkout is one refresh.
                    while rx.recv_timeout(DEBOUNCE).is_ok() {}
                    let Some(inner) = weak.upgrade() else { return };
                    if inner.lock().repo.as_deref() != Some(repo.as_path()) {
                        return;
                    }
                    inner.refresh();
                }
            })
            .ok();
    }
}

fn mock_repo() -> (BranchStatus, Vec<Commit>, Vec<DayCount>) {
    use chrono::{Duration as D, Local, Utc};
    let now = Utc::now();
    let commits = [
        "Add calendar countdown",
        "Media controls in the pop-up",
        "Fix session selection",
    ]
    .iter()
    .enumerate()
    .map(|(i, subject)| Commit {
        hash: format!("{i:040}"),
        short: format!("abc{i}d{i}f"),
        subject: format!("{subject} (MOCK)"),
        author: "Mock Author".into(),
        time: now - D::hours(3 * (i as i64 + 1)),
    })
    .collect();
    let today = Local::now().date_naive();
    let activity = (0..ACTIVITY_DAYS)
        .map(|i| {
            let date = today - D::days(i64::from(ACTIVITY_DAYS - 1 - i));
            DayCount {
                date: date.format("%Y-%m-%d").to_string(),
                count: (i * 7 % 11).saturating_sub(4),
            }
        })
        .collect();
    let branch = BranchStatus {
        branch: Some("main".into()),
        upstream: Some("origin/main".into()),
        ahead: 2,
        behind: 0,
        changes: super::ChangeCounts {
            staged: 1,
            modified: 9,
            untracked: 2,
            conflicted: 0,
        },
    };
    (branch, commits, activity)
}

fn mock_pull_requests() -> PullRequests {
    PullRequests::Ready {
        items: vec![super::PullRequest {
            number: 214,
            title: "Notch shell and activity engine (MOCK)".into(),
            author: "mock-author".into(),
            is_draft: false,
            review_decision: "REVIEW_REQUIRED".into(),
            url: "https://github.com/example/notch/pull/214".into(),
            branch: "feat/notch".into(),
        }],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ignores_build_output_and_object_writes() {
        assert!(is_noise(Path::new(r"C:\repo\node_modules\x\index.js")));
        assert!(is_noise(Path::new(r"C:\repo\src-tauri\target\debug\app.exe")));
        assert!(is_noise(Path::new(r"C:\repo\.git\objects\ab\cdef")));
        assert!(is_noise(Path::new(r"C:\repo\.git\index.lock")));
        assert!(!is_noise(Path::new(r"C:\repo\src\main.rs")));
        assert!(!is_noise(Path::new(r"C:\repo\.git\HEAD")));
        assert!(!is_noise(Path::new(r"C:\repo\.git\refs\heads\main")));
    }
}
