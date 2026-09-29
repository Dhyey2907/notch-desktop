//! Owns the transcript index, its watcher, the hook server, and the Claude Code connection.

use std::{
    collections::HashSet,
    path::PathBuf,
    sync::{mpsc, Arc, Mutex},
    time::{Duration, Instant},
};

use chrono::{DateTime, TimeZone, Utc};
use notify::{RecursiveMode, Watcher};
use tauri::{AppHandle, Manager};

use super::{
    approvals::{self, ApprovalChoice, ApprovalRequest, Approvals, AuditEntry, PermissionPayload},
    connect,
    hooks::{Handlers, HookPayload, HookServer, LiveSessions},
    limits::{self, PlanLimits},
    transcripts::{self, IndexStats, QuotaLimits},
    usage::{self, UsageRange, UsageSummary},
    ClaudeStatus, HooksStatus, IndexStatus, QuotaStatus,
};
use crate::{
    db::Db,
    error::{AppError, AppResult},
    events::{AppEvent, EventEnvelope},
    secrets::{self, Credential},
    state::AppState,
};

/// Quiet period after transcript writes before re-indexing.
const DEBOUNCE: Duration = Duration::from_secs(2);
const QUOTA_KEY: &str = "claude.quota";
const LIMITS_KEY: &str = "claude.limits";

struct IndexState {
    ready: bool,
    last_indexed_at: Option<DateTime<Utc>>,
    quota: Option<(DateTime<Utc>, QuotaLimits)>,
}

struct Inner {
    app: AppHandle,
    /// Dedicated connection, so a long first scan never blocks other database work.
    db: Db,
    projects_dir: PathBuf,
    settings_path: PathBuf,
    live: Mutex<LiveSessions>,
    /// Claude Code processes whose exit is being watched.
    watched_pids: Mutex<HashSet<u32>>,
    approvals: Approvals,
    /// Last plan limits from Claude Code's status line (also cached in `kv_cache`).
    limits: Mutex<Option<PlanLimits>>,
    index: Mutex<IndexState>,
    server: Mutex<Option<HookServer>>,
    watcher: Mutex<Option<notify::RecommendedWatcher>>,
}

#[derive(Clone)]
pub struct ClaudeService {
    inner: Arc<Inner>,
}

impl ClaudeService {
    pub fn new(app: AppHandle, db_path: PathBuf) -> AppResult<Self> {
        let home = app.path().home_dir()?;
        let db = Db::open(&db_path)?;
        let quota = db
            .with(|conn| {
                conn.query_row("SELECT value FROM kv_cache WHERE key = ?1", [QUOTA_KEY], |r| {
                    r.get::<_, String>(0)
                })
                .map_err(Into::into)
            })
            .ok()
            .and_then(|json| serde_json::from_str::<(DateTime<Utc>, String, i64)>(&json).ok())
            .map(|(at, limit_type, resets_at)| {
                (
                    at,
                    QuotaLimits {
                        status: "rejected".into(),
                        rate_limit_type: limit_type,
                        resets_at,
                    },
                )
            });
        let limits = db
            .with(|conn| {
                conn.query_row("SELECT value FROM kv_cache WHERE key = ?1", [LIMITS_KEY], |r| {
                    r.get::<_, String>(0)
                })
                .map_err(Into::into)
            })
            .ok()
            .and_then(|json| serde_json::from_str::<PlanLimits>(&json).ok());
        Ok(Self {
            inner: Arc::new(Inner {
                app,
                db,
                projects_dir: home.join(".claude").join("projects"),
                settings_path: connect::settings_path(&home),
                live: Mutex::new(LiveSessions::default()),
                watched_pids: Mutex::new(HashSet::new()),
                approvals: Approvals::default(),
                limits: Mutex::new(limits),
                index: Mutex::new(IndexState {
                    ready: false,
                    last_indexed_at: None,
                    quota,
                }),
                server: Mutex::new(None),
                watcher: Mutex::new(None),
            }),
        })
    }

    pub fn start(&self) {
        let inner = self.inner.clone();
        std::thread::Builder::new()
            .name("claude-index".into())
            .spawn(move || {
                if inner.hooks_installed() {
                    if let Err(error) = inner.start_server() {
                        tracing::warn!(kind = error.kind(), "claude hook server did not start");
                    }
                }
                let started = Instant::now();
                let files = transcripts::transcript_files(&inner.projects_dir);
                inner.index_paths(&files);
                inner.index.lock().unwrap_or_else(|e| e.into_inner()).ready = true;
                tracing::info!(
                    files = files.len(),
                    ms = started.elapsed().as_millis() as u64,
                    "claude transcripts indexed"
                );
                inner.publish_status();
                inner.publish_usage();
                inner.watch();
            })
            .ok();
    }

    pub fn status(&self) -> ClaudeStatus {
        self.inner.status()
    }

    pub fn usage(&self, range: UsageRange) -> AppResult<UsageSummary> {
        self.inner
            .db
            .with(|conn| usage::summarize(conn, range, Utc::now()))
    }

    pub fn preview(&self) -> String {
        connect::preview()
    }

    /// Add Notch's hooks to Claude Code's settings and start listening. The UI only calls this
    /// after showing `preview()` and getting the user's confirmation.
    pub fn connect(&self) -> AppResult<()> {
        let token = match secrets::get(Credential::ClaudeHookToken)? {
            Some(token) => token,
            None => {
                let token = crate::oauth::random_urlsafe(32);
                secrets::set(Credential::ClaudeHookToken, &token)?;
                token
            }
        };
        let mut settings = connect::read(&self.inner.settings_path)?;
        connect::add_ours(&mut settings, &token)?;
        connect::write(&self.inner.settings_path, &settings)?;
        tracing::info!("claude code hooks installed");
        if self
            .inner
            .server
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .is_none()
        {
            self.inner.start_server()?;
        }
        self.inner.publish_status();
        Ok(())
    }

    pub fn pending_approvals(&self) -> Vec<ApprovalRequest> {
        self.inner.approvals.pending()
    }

    /// Deliver your answer to a waiting permission request.
    pub fn decide(&self, id: &str, choice: ApprovalChoice) -> AppResult<()> {
        if self.inner.approvals.decide(id, choice) {
            Ok(())
        } else {
            Err(AppError::InvalidInput(
                "That request already expired; answer it in the terminal".into(),
            ))
        }
    }

    pub fn approval_audit(&self, limit: u32) -> AppResult<Vec<AuditEntry>> {
        self.inner.db.with(|conn| approvals::recent(conn, limit))
    }

    /// Run a fake permission request through the real approval flow (dev builds only).
    #[cfg(debug_assertions)]
    pub fn dev_test_approval(&self, tool: String, input: serde_json::Value) {
        let inner = self.inner.clone();
        std::thread::spawn(move || {
            let payload = PermissionPayload {
                session_id: "dev-test".into(),
                cwd: Some("C:/dev/notch-desktop".into()),
                tool_name: tool,
                tool_input: input,
            };
            inner.handle_permission(&payload);
        });
    }

    pub fn status_line_preview(&self) -> String {
        connect::status_line_preview()
    }

    /// Add Notch's status line to Claude Code's settings so plan limits reach Notch. Needs
    /// Claude Code connected (it reuses the hook token and server). The UI only calls this
    /// after showing `status_line_preview()` and getting the user's confirmation.
    pub fn enable_status_line(&self) -> AppResult<()> {
        let token = secrets::get(Credential::ClaudeHookToken)?
            .ok_or_else(|| AppError::InvalidInput("Connect Claude Code first".into()))?;
        let mut settings = connect::read(&self.inner.settings_path)?;
        connect::add_status_line(&mut settings, &token)?;
        connect::write(&self.inner.settings_path, &settings)?;
        tracing::info!("claude code status line installed");
        self.inner.publish_status();
        Ok(())
    }

    pub fn disable_status_line(&self) -> AppResult<()> {
        let mut settings = connect::read(&self.inner.settings_path)?;
        if connect::has_our_status_line(&settings) {
            connect::remove_status_line(&mut settings);
            connect::write(&self.inner.settings_path, &settings)?;
        }
        self.inner.set_limits(None);
        tracing::info!("claude code status line removed");
        self.inner.publish_status();
        Ok(())
    }

    pub fn disconnect(&self) -> AppResult<()> {
        let mut settings = connect::read(&self.inner.settings_path)?;
        if connect::is_installed(&settings) || connect::has_our_status_line(&settings) {
            connect::remove_ours(&mut settings);
            connect::remove_status_line(&mut settings);
            connect::write(&self.inner.settings_path, &settings)?;
        }
        self.inner.set_limits(None);
        *self.inner.server.lock().unwrap_or_else(|e| e.into_inner()) = None;
        secrets::delete(Credential::ClaudeHookToken)?;
        *self.inner.live.lock().unwrap_or_else(|e| e.into_inner()) = LiveSessions::default();
        tracing::info!("claude code hooks removed");
        self.inner.publish_status();
        Ok(())
    }
}

impl Inner {
    fn hooks_installed(&self) -> bool {
        connect::read(&self.settings_path)
            .map(|s| connect::is_installed(&s))
            .unwrap_or(false)
    }

    fn status(&self) -> ClaudeStatus {
        let now = Utc::now();
        let live = self.live.lock().unwrap_or_else(|e| e.into_inner()).snapshot(now);
        let index = self.index.lock().unwrap_or_else(|e| e.into_inner());
        let quota = index.quota.as_ref().and_then(|(_, q)| {
            let resets_at = Utc.timestamp_opt(q.resets_at, 0).single()?;
            (resets_at > now).then(|| QuotaStatus {
                limit_type: q.rate_limit_type.clone(),
                resets_at,
            })
        });
        let settings = connect::read(&self.settings_path).ok();
        let check = |f: fn(&serde_json::Value) -> bool| settings.as_ref().is_some_and(f);
        let limits = self
            .limits
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .and_then(|l| l.current(now));
        ClaudeStatus {
            live,
            hooks: HooksStatus {
                installed: check(connect::is_installed),
                outdated: check(connect::is_outdated),
                status_line: check(connect::has_our_status_line),
                status_line_taken: check(connect::status_line_taken),
                listening: self.server.lock().unwrap_or_else(|e| e.into_inner()).is_some(),
            },
            limits,
            index: IndexStatus {
                transcripts_found: self.projects_dir.is_dir(),
                ready: index.ready,
                last_indexed_at: index.last_indexed_at,
            },
            quota,
        }
    }

    /// Start watching a Claude Code process (once) so its sessions end if it's killed.
    fn watch_process(self: &Arc<Self>, pid: u32) {
        if !self
            .watched_pids
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .insert(pid)
        {
            return;
        }
        let weak = Arc::downgrade(self);
        let watching = super::process::watch_exit(pid, move || {
            if let Some(inner) = weak.upgrade() {
                inner.process_exited(pid);
            }
        });
        if !watching {
            // Couldn't open it (already gone?): allow a retry on its next hook.
            self.watched_pids
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .remove(&pid);
        }
    }

    fn process_exited(&self, pid: u32) {
        self.watched_pids
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .remove(&pid);
        let ended = self
            .live
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .process_exited(pid);
        for session in &ended {
            self.approvals.end_session(session);
        }
        tracing::debug!(sessions = ended.len(), "claude code process exited");
        if !ended.is_empty() {
            self.publish_status();
        }
    }

    /// Remember (or forget) the latest plan limits, in memory and in the cache table.
    fn set_limits(&self, limits: Option<PlanLimits>) {
        *self.limits.lock().unwrap_or_else(|e| e.into_inner()) = limits;
        let result = self.db.with(|conn| {
            match limits {
                Some(limits) => {
                    let json = serde_json::to_string(&limits).unwrap_or_default();
                    conn.execute(
                        "INSERT INTO kv_cache (key, value, fetched_at) VALUES (?1, ?2, ?3)
                         ON CONFLICT(key) DO UPDATE SET value = excluded.value, fetched_at = excluded.fetched_at",
                        rusqlite::params![LIMITS_KEY, json, limits.updated_at.to_rfc3339()],
                    )?;
                }
                None => {
                    conn.execute("DELETE FROM kv_cache WHERE key = ?1", [LIMITS_KEY])?;
                }
            }
            Ok(())
        });
        if let Err(error) = result {
            tracing::warn!(kind = error.kind(), "could not cache plan limits");
        }
    }

    /// One status-line update from Claude Code: keep its limits, reply with the line to show.
    fn handle_status_line(&self, body: &[u8]) -> String {
        let now = Utc::now();
        let parsed = limits::parse(body, now);
        tracing::debug!(has_limits = parsed.is_some(), "status line update");
        if let Some(limits) = parsed {
            self.set_limits(Some(limits));
            self.publish_status();
        }
        let current = self
            .limits
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .and_then(|l| l.current(now));
        limits::status_text(current.as_ref())
    }

    fn publish_status(&self) {
        let envelope = EventEnvelope::new("claude", AppEvent::ClaudeStatusChanged(self.status()))
            .with_dedup_key("claude-status");
        self.app.state::<AppState>().bus.publish(envelope);
    }

    /// Tell the UI that usage numbers changed; it re-queries the range it's showing.
    fn publish_usage(&self) {
        self.app
            .state::<AppState>()
            .bus
            .emit("claude", AppEvent::ClaudeUsageChanged);
    }

    fn index_paths(&self, paths: &[PathBuf]) {
        let mut stats = IndexStats::default();
        let result = self.db.with_mut(|conn| {
            for path in paths {
                if let Err(error) = transcripts::index_file(conn, path, &mut stats) {
                    // Transcripts can be mid-write or deleted; skip and retry next time.
                    tracing::debug!(kind = error.kind(), "could not index a transcript");
                }
            }
            Ok(())
        });
        if let Err(error) = result {
            tracing::warn!(kind = error.kind(), "transcript indexing failed");
        }
        let mut index = self.index.lock().unwrap_or_else(|e| e.into_inner());
        index.last_indexed_at = Some(Utc::now());
        if let Some((at, quota)) = stats.latest_quota {
            if index.quota.as_ref().is_none_or(|(prev, _)| at > *prev) {
                let json =
                    serde_json::to_string(&(at, &quota.rate_limit_type, quota.resets_at)).unwrap_or_default();
                let _ = self.db.with(|conn| {
                    conn.execute(
                        "INSERT INTO kv_cache (key, value, fetched_at) VALUES (?1, ?2, ?3)
                         ON CONFLICT(key) DO UPDATE SET value = excluded.value, fetched_at = excluded.fetched_at",
                        rusqlite::params![QUOTA_KEY, json, at.to_rfc3339()],
                    )?;
                    Ok(())
                });
                index.quota = Some((at, quota));
            }
        }
        if stats.records > 0 {
            tracing::debug!(
                files = stats.files_read,
                records = stats.records,
                "claude transcripts updated"
            );
        }
    }

    /// Re-index transcripts as Claude Code appends to them.
    fn watch(self: &Arc<Self>) {
        if !self.projects_dir.is_dir() {
            return;
        }
        let (tx, rx) = mpsc::channel::<PathBuf>();
        let watcher = notify::recommended_watcher(move |result: notify::Result<notify::Event>| {
            if let Ok(event) = result {
                for path in event.paths {
                    if path.extension().is_some_and(|e| e == "jsonl") {
                        let _ = tx.send(path);
                    }
                }
            }
        });
        let Ok(mut watcher) = watcher else { return };
        if watcher
            .watch(&self.projects_dir, RecursiveMode::Recursive)
            .is_err()
        {
            return;
        }
        *self.watcher.lock().unwrap_or_else(|e| e.into_inner()) = Some(watcher);
        let weak = Arc::downgrade(self);
        std::thread::Builder::new()
            .name("claude-watch".into())
            .spawn(move || {
                while let Ok(first) = rx.recv() {
                    let mut changed: HashSet<PathBuf> = HashSet::from([first]);
                    while let Ok(path) = rx.recv_timeout(DEBOUNCE) {
                        changed.insert(path);
                    }
                    let Some(inner) = weak.upgrade() else { return };
                    let paths: Vec<PathBuf> = changed.into_iter().filter(|p| p.is_file()).collect();
                    inner.index_paths(&paths);
                    inner.publish_usage();
                    inner.publish_status();
                }
            })
            .ok();
    }

    /// Ask you about one permission request and return the hook response. Blocks this
    /// request's thread until you answer or the wait runs out.
    fn handle_permission(&self, payload: &PermissionPayload) -> String {
        let request = Approvals::request(payload, Utc::now());
        let outcome = if self.approvals.allowed_by_session_rule(&request) {
            approvals::Outcome {
                behavior: Some("allow"),
                decided_by: "session_rule",
            }
        } else {
            self.approvals
                .wait_for_decision(request.clone(), approvals::DECISION_WAIT, &|| {
                    self.publish_approvals()
                })
        };
        // Only the decision is logged; the command itself goes to the audit table.
        tracing::info!(
            decision = outcome.behavior.unwrap_or("ask"),
            decided_by = outcome.decided_by,
            destructive = request.destructive,
            "permission request answered"
        );
        if let Err(error) = self.db.with(|conn| approvals::record(conn, &request, &outcome)) {
            tracing::warn!(kind = error.kind(), "could not record approval audit");
        }
        approvals::response_body(outcome.behavior)
    }

    fn resolve_elsewhere(&self, session_id: &str, tool: Option<(&str, &str)>) {
        let closed = self.approvals.resolve_elsewhere(session_id, tool);
        if closed > 0 {
            tracing::debug!(closed, "approval cards answered in claude code");
        }
    }

    fn publish_approvals(&self) {
        self.app
            .state::<AppState>()
            .bus
            .emit("claude", AppEvent::ApprovalsChanged(self.approvals.pending()));
    }

    fn start_server(self: &Arc<Self>) -> AppResult<()> {
        let token = secrets::get(Credential::ClaudeHookToken)?
            .ok_or_else(|| AppError::InvalidInput("Claude Code isn't connected".into()))?;
        let weak = Arc::downgrade(self);
        let handler = Arc::new(move |payload: HookPayload, body: &[u8]| {
            let Some(inner) = weak.upgrade() else {
                return approvals::response_body(None);
            };
            let changed = inner
                .live
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .apply(&payload, Utc::now());
            if changed {
                inner.publish_status();
            }
            if let Some(pid) = payload.client_pid {
                inner.watch_process(pid);
            }
            match payload.hook_event_name.as_str() {
                "PermissionRequest" => match serde_json::from_slice::<PermissionPayload>(body) {
                    // Questions and plan approvals have their own dialog in Claude Code.
                    Ok(request) if approvals::is_user_dialog(&request.tool_name) => {
                        approvals::response_body(None)
                    }
                    Ok(request) => inner.handle_permission(&request),
                    Err(_) => approvals::response_body(None),
                },
                // The tool ran (or failed) after being answered in Claude Code itself.
                "PostToolUse" | "PostToolUseFailure" => {
                    if let Ok(done) = serde_json::from_slice::<PermissionPayload>(body) {
                        let summary = approvals::summarize(&done.tool_name, &done.tool_input);
                        inner.resolve_elsewhere(&done.session_id, Some((&done.tool_name, &summary)));
                    }
                    "{}".to_owned()
                }
                // The turn moved on, so anything still open was answered in Claude Code.
                "UserPromptSubmit" | "Stop" => {
                    inner.resolve_elsewhere(&payload.session_id, None);
                    "{}".to_owned()
                }
                "SessionEnd" => {
                    inner.approvals.end_session(&payload.session_id);
                    "{}".to_owned()
                }
                _ => "{}".to_owned(),
            }
        });
        let weak = Arc::downgrade(self);
        let status_line = Arc::new(move |body: &[u8]| {
            weak.upgrade()
                .map(|inner| inner.handle_status_line(body))
                .unwrap_or_default()
        });
        let handlers = Handlers {
            hook: handler,
            status_line,
        };
        let server = HookServer::start(super::hooks::HOOK_PORT, token, handlers).map_err(|e| {
            AppError::InvalidInput(format!(
                "port {} is busy, so live activity can't start ({e})",
                super::hooks::HOOK_PORT
            ))
        })?;
        *self.server.lock().unwrap_or_else(|e| e.into_inner()) = Some(server);
        tracing::info!("claude hook server listening");
        Ok(())
    }
}
