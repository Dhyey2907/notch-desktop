//! GitHub device-flow sign-in, pull requests over the GraphQL API, and notifications
//! (review requests, CI results, mentions) polled while signed in.

use std::{
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, Mutex,
    },
    time::Duration,
};

use chrono::Utc;
use serde_json::{json, Value};
use tauri::{AppHandle, Manager};

use super::{
    notifications::{self, GithubNotification, Seen},
    parse_commits, parse_repos, parse_token_poll, DeviceCode, DeviceCodeResponse, GithubRepo,
    GithubRepoOverview, GithubStatus, TokenPoll,
};
use crate::{
    error::{AppError, AppResult},
    events::AppEvent,
    git::{parse_prs, PrUnavailable, PullRequests},
    oauth_clients,
    secrets::{self, Credential},
    state::AppState,
};

const DEVICE_CODE_URL: &str = "https://github.com/login/device/code";
const TOKEN_URL: &str = "https://github.com/login/oauth/access_token";
const API: &str = "https://api.github.com";
/// `repo` lets Notch list pull requests in your private repositories too (read only here).
const SCOPE: &str = "repo read:user";
const USER_AGENT: &str = "Overline";

const PR_QUERY: &str = "query($owner: String!, $name: String!) {
  repository(owner: $owner, name: $name) {
    pullRequests(states: OPEN, first: 6, orderBy: {field: UPDATED_AT, direction: DESC}) {
      nodes { number title isDraft reviewDecision url headRefName author { login } }
    }
  }
}";

const REPOS_QUERY: &str = "query {
  viewer {
    repositories(first: 60, orderBy: {field: PUSHED_AT, direction: DESC},
                 affiliations: [OWNER, COLLABORATOR, ORGANIZATION_MEMBER],
                 ownerAffiliations: [OWNER, COLLABORATOR, ORGANIZATION_MEMBER]) {
      nodes { nameWithOwner description isPrivate pushedAt url }
    }
  }
}";

const OVERVIEW_QUERY: &str = "query($owner: String!, $name: String!) {
  repository(owner: $owner, name: $name) {
    nameWithOwner description isPrivate pushedAt url
    defaultBranchRef {
      name
      target { ... on Commit { history(first: 6) {
        nodes { messageHeadline committedDate url author { name user { login } } }
      } } }
    }
    pullRequests(states: OPEN, first: 6, orderBy: {field: UPDATED_AT, direction: DESC}) {
      nodes { number title isDraft reviewDecision url headRefName author { login } }
    }
  }
}";

/// Why a GitHub API call gave nothing usable.
enum ApiError {
    /// Not signed in, or the token was revoked (it's forgotten).
    SignedOut,
    Failed,
}

struct Inner {
    app: AppHandle,
    http: reqwest::Client,
    status: Mutex<GithubStatus>,
    /// Bumped to cancel a sign-in that's waiting for approval.
    attempt: AtomicU64,
    /// Unread notifications, newest first.
    notifications: Mutex<Vec<GithubNotification>>,
    /// Wakes the notifications poller (signed in or out).
    wake: tokio::sync::Notify,
}

/// GitHub asks clients to poll notifications no more often than this (`X-Poll-Interval`).
const MIN_POLL: Duration = Duration::from_secs(60);
/// Back off after errors (offline, GitHub down).
const ERROR_POLL: Duration = Duration::from_secs(300);

enum Fetched {
    NotModified,
    List(Vec<GithubNotification>),
}

#[derive(Clone)]
pub struct GithubService {
    inner: Arc<Inner>,
}

impl GithubService {
    pub fn new(app: AppHandle) -> Self {
        let signed_in = secrets::get(Credential::GithubToken).ok().flatten().is_some();
        Self {
            inner: Arc::new(Inner {
                app,
                http: reqwest::Client::builder()
                    .timeout(Duration::from_secs(20))
                    .user_agent(USER_AGENT)
                    .build()
                    .expect("static client config"),
                status: Mutex::new(GithubStatus {
                    available: oauth_clients::github().is_some(),
                    signed_in,
                    login: None,
                    pending: None,
                    last_error: None,
                }),
                attempt: AtomicU64::new(0),
                notifications: Mutex::new(Vec::new()),
                wake: tokio::sync::Notify::new(),
            }),
        }
    }

    /// Fill in the account name for an existing sign-in, and watch notifications.
    pub fn start(&self) {
        let poller = self.inner.clone();
        tauri::async_runtime::spawn(async move { poller.watch_notifications().await });
        if !self.inner.status().signed_in {
            return;
        }
        let inner = self.inner.clone();
        tauri::async_runtime::spawn(async move {
            if let Ok(Some(token)) = secrets::get(Credential::GithubToken) {
                let login = inner.fetch_login(&token).await;
                inner.update(|s| s.login = login);
            }
        });
    }

    pub fn status(&self) -> GithubStatus {
        self.inner.status()
    }

    /// Start signing in: returns the code to enter on github.com (also opened in the browser)
    /// and waits for approval in the background.
    pub async fn sign_in(&self) -> AppResult<DeviceCode> {
        let client_id = oauth_clients::github()
            .ok_or_else(|| AppError::InvalidInput("This build has no GitHub sign-in configured".into()))?;
        let response: DeviceCodeResponse = self
            .inner
            .http
            .post(DEVICE_CODE_URL)
            .header("Accept", "application/json")
            .form(&[("client_id", client_id), ("scope", SCOPE)])
            .send()
            .await
            .map_err(|_| AppError::InvalidInput("Couldn't reach GitHub. Check your connection.".into()))?
            .error_for_status()
            .map_err(|_| AppError::InvalidInput("GitHub refused the sign-in request.".into()))?
            .json()
            .await
            .map_err(|_| AppError::InvalidInput("GitHub sent an unexpected answer.".into()))?;
        if !response.verification_uri.starts_with("https://github.com/") {
            return Err(AppError::InvalidInput(
                "GitHub sent an unexpected sign-in page.".into(),
            ));
        }
        let code = DeviceCode {
            user_code: response.user_code.clone(),
            verification_uri: response.verification_uri.clone(),
            expires_at: Utc::now() + chrono::Duration::seconds(response.expires_in),
        };
        let attempt = self.inner.attempt.fetch_add(1, Ordering::SeqCst) + 1;
        self.inner.update(|s| {
            s.pending = Some(code.clone());
            s.last_error = None;
        });
        {
            use tauri_plugin_opener::OpenerExt;
            let _ = self
                .inner
                .app
                .opener()
                .open_url(&response.verification_uri, None::<&str>);
        }
        let inner = self.inner.clone();
        tauri::async_runtime::spawn(
            async move { inner.wait_for_approval(client_id, response, attempt).await },
        );
        Ok(code)
    }

    pub fn cancel_sign_in(&self) {
        self.inner.attempt.fetch_add(1, Ordering::SeqCst);
        self.inner.update(|s| s.pending = None);
    }

    pub fn sign_out(&self) -> AppResult<()> {
        self.inner.attempt.fetch_add(1, Ordering::SeqCst);
        secrets::delete(Credential::GithubToken)?;
        self.inner.update(|s| {
            s.signed_in = false;
            s.login = None;
            s.pending = None;
        });
        tracing::info!("github signed out");
        Ok(())
    }

    pub fn notifications(&self) -> Vec<GithubNotification> {
        self.inner
            .notifications
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    /// Mark a notification read on GitHub and drop it from the list.
    pub async fn mark_read(&self, id: &str) -> AppResult<()> {
        if id.is_empty() || id.len() > 32 || !id.chars().all(|c| c.is_ascii_digit()) {
            return Err(AppError::InvalidInput("invalid notification".into()));
        }
        let Ok(Some(token)) = secrets::get(Credential::GithubToken) else {
            return Err(AppError::InvalidInput("Sign in with GitHub first".into()));
        };
        let response = self
            .inner
            .http
            .patch(format!("{API}/notifications/threads/{id}"))
            .bearer_auth(&token)
            .header("Accept", "application/vnd.github+json")
            .send()
            .await
            .map_err(|_| AppError::InvalidInput("Couldn't reach GitHub".into()))?;
        if !response.status().is_success() {
            return Err(AppError::InvalidInput("GitHub didn't mark it read".into()));
        }
        let list = {
            let mut list = self.inner.notifications.lock().unwrap_or_else(|e| e.into_inner());
            list.retain(|n| n.id != id);
            list.clone()
        };
        self.inner.publish_notifications(list);
        Ok(())
    }

    pub fn signed_in(&self) -> bool {
        self.inner.status().signed_in
    }

    /// Run a GraphQL query with the stored token.
    async fn graphql(&self, query: &str, variables: Value) -> Result<Value, ApiError> {
        let Ok(Some(token)) = secrets::get(Credential::GithubToken) else {
            return Err(ApiError::SignedOut);
        };
        let response = self
            .inner
            .http
            .post(format!("{API}/graphql"))
            .bearer_auth(&token)
            .json(&json!({ "query": query, "variables": variables }))
            .send()
            .await
            .map_err(|_| ApiError::Failed)?;
        if response.status() == reqwest::StatusCode::UNAUTHORIZED {
            // Revoked on github.com: forget it so Settings offers signing in again.
            let _ = self.sign_out();
            return Err(ApiError::SignedOut);
        }
        if !response.status().is_success() {
            return Err(ApiError::Failed);
        }
        response.json::<Value>().await.map_err(|_| ApiError::Failed)
    }

    fn pull_requests_from(repository: Option<&Value>) -> PullRequests {
        let unavailable = |reason| PullRequests::Unavailable { reason };
        let Some(nodes) = repository.and_then(|r| r.pointer("/pullRequests/nodes")) else {
            // No such repository, or no access to it with this sign-in.
            return unavailable(PrUnavailable::NotGithub);
        };
        match parse_prs(&nodes.to_string()) {
            Some(items) => PullRequests::Ready { items },
            None => unavailable(PrUnavailable::Failed),
        }
    }

    /// Open pull requests for `owner/name`, newest activity first.
    pub async fn pull_requests(&self, owner: &str, name: &str) -> PullRequests {
        match self
            .graphql(PR_QUERY, json!({ "owner": owner, "name": name }))
            .await
        {
            Ok(body) => Self::pull_requests_from(body.pointer("/data/repository")),
            Err(ApiError::SignedOut) => PullRequests::Unavailable {
                reason: PrUnavailable::NotAuthenticated,
            },
            Err(ApiError::Failed) => PullRequests::Unavailable {
                reason: PrUnavailable::Failed,
            },
        }
    }

    /// Your repositories, most recently pushed first.
    pub async fn repositories(&self) -> AppResult<Vec<GithubRepo>> {
        match self.graphql(REPOS_QUERY, json!({})).await {
            Ok(body) => Ok(body
                .pointer("/data/viewer/repositories/nodes")
                .map(parse_repos)
                .unwrap_or_default()),
            Err(ApiError::SignedOut) => Err(AppError::InvalidInput("Sign in with GitHub first".into())),
            Err(ApiError::Failed) => Err(AppError::InvalidInput("Couldn't reach GitHub. Try again.".into())),
        }
    }

    /// A repository's latest commits and open pull requests, straight from GitHub.
    pub async fn overview(&self, name_with_owner: &str) -> AppResult<GithubRepoOverview> {
        let (owner, name) = name_with_owner
            .split_once('/')
            .filter(|_| super::valid_name_with_owner(name_with_owner))
            .ok_or_else(|| AppError::InvalidInput("that isn't a GitHub repository name".into()))?;
        let body = match self
            .graphql(OVERVIEW_QUERY, json!({ "owner": owner, "name": name }))
            .await
        {
            Ok(body) => body,
            Err(ApiError::SignedOut) => {
                return Err(AppError::InvalidInput("Sign in with GitHub first".into()))
            }
            Err(ApiError::Failed) => {
                return Err(AppError::InvalidInput("Couldn't reach GitHub. Try again.".into()))
            }
        };
        let repository = body
            .pointer("/data/repository")
            .filter(|r| !r.is_null())
            .ok_or_else(|| {
                AppError::InvalidInput("GitHub couldn't find that repository (or you lost access)".into())
            })?;
        let repo = parse_repos(&json!([repository]))
            .pop()
            .ok_or_else(|| AppError::InvalidInput("GitHub sent an unexpected answer.".into()))?;
        Ok(GithubRepoOverview {
            repo,
            default_branch: repository
                .pointer("/defaultBranchRef/name")
                .and_then(|n| n.as_str())
                .map(str::to_owned),
            commits: repository
                .pointer("/defaultBranchRef/target/history/nodes")
                .map(parse_commits)
                .unwrap_or_default(),
            pull_requests: Self::pull_requests_from(Some(repository)),
        })
    }
}

impl Inner {
    fn status(&self) -> GithubStatus {
        self.status.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }

    fn update(&self, change: impl FnOnce(&mut GithubStatus)) {
        let status = {
            let mut status = self.status.lock().unwrap_or_else(|e| e.into_inner());
            change(&mut status);
            status.clone()
        };
        self.app
            .state::<AppState>()
            .bus
            .emit("github", AppEvent::GithubChanged(status));
        // Signed in or out: the notifications poller starts or stops.
        self.wake.notify_one();
    }

    fn publish_notifications(&self, list: Vec<GithubNotification>) {
        self.app
            .state::<AppState>()
            .bus
            .emit("github", AppEvent::GithubNotificationsChanged(list));
    }

    /// Poll notifications while signed in, at GitHub's interval; sleep while signed out.
    async fn watch_notifications(self: Arc<Self>) {
        let mut seen = Seen::default();
        let mut last_modified: Option<String> = None;
        loop {
            if !self.status().signed_in {
                seen.reset();
                last_modified = None;
                let had = std::mem::take(&mut *self.notifications.lock().unwrap_or_else(|e| e.into_inner()));
                if !had.is_empty() {
                    self.publish_notifications(Vec::new());
                }
                self.wake.notified().await;
                continue;
            }
            let (fetched, poll) = self.fetch_notifications(&mut last_modified).await;
            if let Some(Fetched::List(list)) = fetched {
                for alert in seen.fresh(&list) {
                    self.app
                        .state::<AppState>()
                        .bus
                        .emit("github", AppEvent::GithubAlert(alert));
                }
                *self.notifications.lock().unwrap_or_else(|e| e.into_inner()) = list.clone();
                self.publish_notifications(list);
            }
            tokio::select! {
                _ = tokio::time::sleep(poll) => {}
                _ = self.wake.notified() => {}
            }
        }
    }

    /// One `GET /notifications`. `If-Modified-Since` makes unchanged checks free (304).
    async fn fetch_notifications(&self, last_modified: &mut Option<String>) -> (Option<Fetched>, Duration) {
        let Ok(Some(token)) = secrets::get(Credential::GithubToken) else {
            return (None, ERROR_POLL);
        };
        let mut request = self
            .http
            .get(format!(
                "{API}/notifications?per_page={}",
                notifications::MAX_ITEMS
            ))
            .bearer_auth(&token)
            .header("Accept", "application/vnd.github+json");
        if let Some(since) = last_modified.as_deref() {
            request = request.header("If-Modified-Since", since);
        }
        let Ok(response) = request.send().await else {
            return (None, ERROR_POLL);
        };
        let poll = response
            .headers()
            .get("X-Poll-Interval")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.parse::<u64>().ok())
            .map(Duration::from_secs)
            .unwrap_or(MIN_POLL)
            .clamp(MIN_POLL, Duration::from_secs(3600));
        match response.status() {
            reqwest::StatusCode::NOT_MODIFIED => (Some(Fetched::NotModified), poll),
            reqwest::StatusCode::UNAUTHORIZED => {
                // Revoked on github.com: forget it so Settings offers signing in again.
                tracing::info!("github rejected the saved sign-in; signed out");
                let _ = secrets::delete(Credential::GithubToken);
                self.update(|s| {
                    s.signed_in = false;
                    s.login = None;
                });
                (None, poll)
            }
            status if status.is_success() => {
                let modified = response
                    .headers()
                    .get("Last-Modified")
                    .and_then(|v| v.to_str().ok())
                    .map(str::to_owned);
                match response.json::<Value>().await {
                    Ok(json) => {
                        *last_modified = modified;
                        (Some(Fetched::List(notifications::parse(&json))), poll)
                    }
                    Err(_) => (None, ERROR_POLL),
                }
            }
            _ => (None, ERROR_POLL),
        }
    }

    async fn fetch_login(&self, token: &str) -> Option<String> {
        let user: Value = self
            .http
            .get(format!("{API}/user"))
            .bearer_auth(token)
            .send()
            .await
            .ok()?
            .error_for_status()
            .ok()?
            .json()
            .await
            .ok()?;
        user.get("login")?.as_str().map(|l| l.chars().take(64).collect())
    }

    /// Poll GitHub at the interval it asks for — only while a sign-in is waiting for you.
    async fn wait_for_approval(self: Arc<Self>, client_id: &str, code: DeviceCodeResponse, attempt: u64) {
        let mut interval = code.interval.clamp(1, 60);
        let deadline =
            tokio::time::Instant::now() + Duration::from_secs(code.expires_in.clamp(60, 1800) as u64);
        let outcome = loop {
            tokio::time::sleep(Duration::from_secs(interval)).await;
            if self.attempt.load(Ordering::SeqCst) != attempt {
                return; // cancelled or superseded
            }
            if tokio::time::Instant::now() >= deadline {
                break Err("The code expired. Try again.");
            }
            let poll = self
                .http
                .post(TOKEN_URL)
                .header("Accept", "application/json")
                .form(&[
                    ("client_id", client_id),
                    ("device_code", code.device_code.as_str()),
                    ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
                ])
                .send()
                .await;
            let Ok(response) = poll else { continue }; // offline for a moment: keep waiting
            let Ok(json) = response.json::<Value>().await else {
                continue;
            };
            match parse_token_poll(&json) {
                TokenPoll::Token(token) => break Ok(token),
                TokenPoll::Pending { slow_down } => {
                    if slow_down {
                        interval += 5;
                    }
                }
                TokenPoll::Denied => break Err("Sign-in was cancelled on GitHub."),
                TokenPoll::Expired => break Err("The code expired. Try again."),
                TokenPoll::Failed(_) => break Err("GitHub sign-in failed. Try again."),
            }
        };
        if self.attempt.load(Ordering::SeqCst) != attempt {
            return;
        }
        match outcome {
            Ok(token) => {
                if secrets::set(Credential::GithubToken, &token).is_err() {
                    self.update(|s| {
                        s.pending = None;
                        s.last_error =
                            Some("Couldn't save the sign-in to Windows Credential Manager.".into());
                    });
                    return;
                }
                let login = self.fetch_login(&token).await;
                tracing::info!("github signed in");
                self.update(|s| {
                    s.pending = None;
                    s.signed_in = true;
                    s.login = login;
                    s.last_error = None;
                });
            }
            Err(message) => {
                tracing::info!("github sign-in did not complete");
                self.update(|s| {
                    s.pending = None;
                    s.last_error = Some(message.into());
                });
            }
        }
    }
}
