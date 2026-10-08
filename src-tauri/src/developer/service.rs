//! Watches local dev servers (every 10 s) and counts coding minutes (once a minute, while an
//! editor is in front and the PC is in use).

use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use chrono::{Duration as Days, Local};
use tauri::{AppHandle, Manager};

use super::{runner, widgets, CodingStats, DevServer, ProjectTime};
use crate::{
    error::{AppError, AppResult},
    events::AppEvent,
    state::AppState,
};

const SCAN_EVERY: Duration = Duration::from_secs(10);
/// A server that ran at least this long gets a note when it stops.
const NOTICE_AFTER: Duration = Duration::from_secs(30);
/// Input within this long counts as "in use".
const ACTIVE_MS: u64 = 120_000;

#[derive(Default)]
struct Inner {
    servers: Vec<DevServer>,
    /// When each (port, pid) was first seen.
    since: HashMap<(u16, u32), Instant>,
    command_lines: HashMap<u32, Option<String>>,
    /// Stopped from Notch: no "it stopped" note for these.
    stopped_here: HashSet<u32>,
}

#[derive(Clone)]
pub struct DeveloperService {
    app: AppHandle,
    inner: Arc<Mutex<Inner>>,
}

impl DeveloperService {
    pub fn new(app: AppHandle) -> Self {
        Self {
            app,
            inner: Arc::default(),
        }
    }

    fn settings(&self) -> crate::settings::DevSettings {
        self.app
            .state::<AppState>()
            .settings
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .dev
            .clone()
    }

    fn emit(&self, event: AppEvent) {
        self.app.state::<AppState>().bus.emit("developer", event);
    }

    pub fn start(&self) {
        let this = self.clone();
        std::thread::Builder::new()
            .name("developer".into())
            .spawn(move || {
                let mut last_minute = Instant::now();
                loop {
                    this.scan();
                    if last_minute.elapsed() >= Duration::from_secs(60) {
                        last_minute = Instant::now();
                        this.count_minute();
                    }
                    std::thread::sleep(SCAN_EVERY);
                }
            })
            .ok();
    }

    pub fn servers(&self) -> Vec<DevServer> {
        self.inner
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .servers
            .clone()
    }

    /// Look for dev servers now; tell the UI if the list changed.
    pub fn scan(&self) -> Vec<DevServer> {
        #[cfg(windows)]
        {
            let own = std::process::id();
            let listening: Vec<(u16, u32, String)> = super::ports::listeners()
                .into_iter()
                .filter(|(port, pid)| *pid != own && *pid > 4 && *port >= 1024)
                .filter_map(|(port, pid)| {
                    let exe = crate::clipboard::win::exe_of(pid)?;
                    let stem = std::path::Path::new(&exe)
                        .file_stem()?
                        .to_string_lossy()
                        .into_owned();
                    super::is_dev_process(&stem).then_some((port, pid, stem.to_lowercase()))
                })
                .collect();
            let unknown: Vec<u32> = {
                let inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
                let mut pids: Vec<u32> = listening
                    .iter()
                    .map(|(_, pid, _)| *pid)
                    .filter(|pid| !inner.command_lines.contains_key(pid))
                    .collect();
                pids.dedup();
                pids
            };
            let lines = super::ports::command_lines(&unknown);
            let now = Instant::now();
            let (changed, stopped, servers) = {
                let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
                for pid in unknown {
                    let line = lines.get(&pid).and_then(|l| super::short_command(l));
                    inner.command_lines.insert(pid, line);
                }
                let mut servers: Vec<DevServer> = listening
                    .iter()
                    .map(|(port, pid, process)| DevServer {
                        port: *port,
                        pid: *pid,
                        process: process.clone(),
                        command: inner.command_lines.get(pid).cloned().flatten(),
                        url: format!("http://localhost:{port}"),
                    })
                    .collect();
                servers.sort_by_key(|s| s.port);
                servers.dedup_by_key(|s| s.port);
                let live: HashSet<(u16, u32)> = servers.iter().map(|s| (s.port, s.pid)).collect();
                let stopped: Vec<DevServer> = inner
                    .servers
                    .iter()
                    .filter(|s| !live.contains(&(s.port, s.pid)))
                    .filter(|s| !inner.stopped_here.contains(&s.pid))
                    .filter(|s| {
                        inner
                            .since
                            .get(&(s.port, s.pid))
                            .is_some_and(|t| now.duration_since(*t) >= NOTICE_AFTER)
                    })
                    .cloned()
                    .collect();
                for s in &servers {
                    inner.since.entry((s.port, s.pid)).or_insert(now);
                }
                inner.since.retain(|k, _| live.contains(k));
                let pids: HashSet<u32> = servers.iter().map(|s| s.pid).collect();
                inner.command_lines.retain(|pid, _| pids.contains(pid));
                inner.stopped_here.retain(|pid| pids.contains(pid));
                let changed = inner.servers != servers;
                inner.servers = servers.clone();
                (changed, stopped, servers)
            };
            if changed {
                self.emit(AppEvent::DevServersChanged(servers.clone()));
            }
            if !self.settings().no_server_alerts {
                for s in stopped {
                    self.emit(AppEvent::DevServerStopped {
                        port: s.port,
                        name: s.command.unwrap_or(s.process),
                    });
                }
            }
            servers
        }
        #[cfg(not(windows))]
        Vec::new()
    }

    /// Stop a dev server's process (only one Notch listed).
    pub fn stop(&self, pid: u32) -> AppResult<()> {
        let known = self.servers().iter().any(|s| s.pid == pid);
        if !known {
            return Err(AppError::InvalidInput(
                "That server isn't running any more".into(),
            ));
        }
        self.inner
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .stopped_here
            .insert(pid);
        #[cfg(windows)]
        if !super::ports::kill(pid) {
            return Err(AppError::InvalidInput(
                "Windows didn't let Overline stop it".into(),
            ));
        }
        std::thread::sleep(Duration::from_millis(300));
        self.scan();
        Ok(())
    }

    fn db<T>(&self, f: impl FnOnce(&rusqlite::Connection) -> AppResult<T>) -> AppResult<T> {
        self.app.state::<AppState>().db.with(f)
    }

    fn count_minute(&self) {
        if self.settings().no_coding_tracker {
            return;
        }
        #[cfg(windows)]
        {
            use crate::system::screentime::probe;
            if probe::idle_ms() > ACTIVE_MS || probe::locked() {
                return;
            }
            let Some(project) = front_project() else { return };
            let day = Local::now().format("%Y-%m-%d").to_string();
            let saved = self.db(|conn| {
                conn.execute(
                    "INSERT INTO coding_minutes (day, project, minutes) VALUES (?1, ?2, 1)
                     ON CONFLICT(day, project) DO UPDATE SET minutes = minutes + 1",
                    rusqlite::params![day, project],
                )?;
                Ok(())
            });
            if let Err(error) = saved {
                tracing::warn!(kind = error.kind(), "could not save coding time");
            }
        }
    }

    pub fn stats(&self) -> AppResult<CodingStats> {
        let today = Local::now().date_naive();
        let days: Vec<String> = (0..60)
            .map(|d| (today - Days::days(d)).format("%Y-%m-%d").to_string())
            .collect();
        let (by_day, today_rows) = self.db(|conn| {
            let mut stmt =
                conn.prepare("SELECT day, SUM(minutes) FROM coding_minutes WHERE day >= ?1 GROUP BY day")?;
            let by_day: HashMap<String, u32> = stmt
                .query_map([days.last().cloned().unwrap_or_default()], |r| {
                    Ok((r.get(0)?, r.get(1)?))
                })?
                .collect::<Result<_, _>>()?;
            let mut stmt = conn.prepare(
                "SELECT project, minutes FROM coding_minutes WHERE day = ?1 ORDER BY minutes DESC",
            )?;
            let today_rows = stmt
                .query_map([&days[0]], |r| {
                    Ok(ProjectTime {
                        project: r.get(0)?,
                        minutes: r.get(1)?,
                    })
                })?
                .collect::<Result<Vec<_>, _>>()?;
            Ok((by_day, today_rows))
        })?;
        let newest_first: Vec<u32> = days.iter().map(|d| by_day.get(d).copied().unwrap_or(0)).collect();
        Ok(CodingStats {
            today: today_rows,
            week: newest_first.iter().take(7).rev().copied().collect(),
            streak: super::streak(&newest_first),
        })
    }

    pub fn run(&self, language: runner::Language, code: &str) -> AppResult<runner::RunResult> {
        runner::run(language, code)
    }

    pub async fn widgets(&self) -> Vec<widgets::WidgetOutput> {
        let configs = self.settings().widgets;
        let mut out = Vec::new();
        for config in &configs {
            out.push(widgets::load(config).await);
        }
        out
    }
}

/// The project in the editor in front, if an editor is in front.
#[cfg(windows)]
fn front_project() -> Option<String> {
    use windows::Win32::UI::WindowsAndMessaging::{
        GetForegroundWindow, GetWindowTextW, GetWindowThreadProcessId,
    };
    // SAFETY: read-only queries on the foreground window into a local buffer.
    let (pid, title) = unsafe {
        let hwnd = GetForegroundWindow();
        let mut pid = 0;
        GetWindowThreadProcessId(hwnd, Some(&mut pid));
        let mut buf = [0u16; 512];
        let len = GetWindowTextW(hwnd, &mut buf) as usize;
        (pid, String::from_utf16_lossy(&buf[..len]))
    };
    let exe = crate::clipboard::win::exe_of(pid)?;
    let stem = std::path::Path::new(&exe)
        .file_stem()?
        .to_string_lossy()
        .into_owned();
    if !super::is_editor(&stem) {
        return None;
    }
    super::project_from_title(&stem, &title)
}
