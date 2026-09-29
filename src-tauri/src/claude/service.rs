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
    connect,
    hooks::{HookPayload, HookServer, LiveSessions},
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
        Ok(Self {
            inner: Arc::new(Inner {
                app,
                db,
                projects_dir: home.join(".claude").join("projects"),
                settings_path: connect::settings_path(&home),
                live: Mutex::new(LiveSessions::default()),
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

    pub fn disconnect(&self) -> AppResult<()> {
        let mut settings = connect::read(&self.inner.settings_path)?;
        if connect::is_installed(&settings) {
            connect::remove_ours(&mut settings);
            connect::write(&self.inner.settings_path, &settings)?;
        }
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
        ClaudeStatus {
            live,
            hooks: HooksStatus {
                installed: self.hooks_installed(),
                listening: self.server.lock().unwrap_or_else(|e| e.into_inner()).is_some(),
            },
            index: IndexStatus {
                transcripts_found: self.projects_dir.is_dir(),
                ready: index.ready,
                last_indexed_at: index.last_indexed_at,
            },
            quota,
        }
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

    fn start_server(self: &Arc<Self>) -> AppResult<()> {
        let token = secrets::get(Credential::ClaudeHookToken)?
            .ok_or_else(|| AppError::InvalidInput("Claude Code isn't connected".into()))?;
        let weak = Arc::downgrade(self);
        let handler = Arc::new(move |payload: HookPayload| {
            if let Some(inner) = weak.upgrade() {
                let changed = inner
                    .live
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .apply(&payload, Utc::now());
                if changed {
                    inner.publish_status();
                }
            }
            "{}".to_owned()
        });
        let server = HookServer::start(super::hooks::HOOK_PORT, token, handler).map_err(|e| {
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
