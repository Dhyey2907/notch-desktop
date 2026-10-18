//! Desk actions, the live mini view, and routines (run on click or on their schedule).

use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::Duration,
};

use chrono::{Datelike, Local, NaiveDate, Timelike};
use tauri::{AppHandle, Manager, WebviewWindow};

use super::{DeskWindow, Routine, RoutineStep};
use crate::{
    error::{AppError, AppResult},
    events::AppEvent,
    state::AppState,
};

/// A window switcher tile: source window and its rect (physical pixels, island window).
pub type GridTile = (i64, (i32, i32, i32, i32));

const SCHEDULE_TICK: Duration = Duration::from_secs(30);
/// A short pause between steps, so apps don't all start in the same instant.
const STEP_GAP: Duration = Duration::from_millis(350);

/// The rolling "last session" snapshot is refreshed this often.
const SESSION_EVERY: Duration = Duration::from_secs(5 * 60);
/// "Reopen last session" is offered only for a session this recent.
const RESUME_WITHIN_SECS: i64 = 3 * 24 * 60 * 60;

#[derive(Default)]
struct Inner {
    /// Island label → (thumbnail id, source window).
    thumbnails: HashMap<String, (isize, i64)>,
    /// Island label → the window switcher's live views.
    grid: HashMap<String, Vec<isize>>,
    /// The resume offer is made once per run.
    offered: bool,
    /// Routine id → the day it last ran by schedule.
    ran: HashMap<String, NaiveDate>,
}

#[derive(Clone)]
pub struct DeskService {
    app: AppHandle,
    inner: Arc<Mutex<Inner>>,
}

impl DeskService {
    pub fn new(app: AppHandle) -> Self {
        Self {
            app,
            inner: Arc::default(),
        }
    }

    fn emit(&self, event: AppEvent) {
        self.app.state::<AppState>().bus.emit("desk", event);
    }

    fn routines(&self) -> Vec<Routine> {
        self.app
            .state::<AppState>()
            .settings
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .desk
            .routines
            .clone()
    }

    pub fn start(&self) {
        let this = self.clone();
        std::thread::Builder::new()
            .name("routines".into())
            .spawn(move || loop {
                std::thread::sleep(SCHEDULE_TICK);
                let now = Local::now();
                let today = now.date_naive();
                let weekday = now.weekday().number_from_monday() as u8;
                let minute = now.hour() * 60 + now.minute();
                this.maybe_sleep(today, weekday, minute);
                for routine in this.routines() {
                    let ran = this
                        .inner
                        .lock()
                        .unwrap_or_else(|e| e.into_inner())
                        .ran
                        .get(&routine.id)
                        .is_some_and(|d| *d == today);
                    if super::due(&routine, weekday, minute, ran) {
                        this.inner
                            .lock()
                            .unwrap_or_else(|e| e.into_inner())
                            .ran
                            .insert(routine.id.clone(), today);
                        let _ = this.run(&routine.id, true);
                    }
                }
            })
            .ok();
    }

    /// The power schedule's sleep: at the time, on the day, once you've been away a while.
    fn maybe_sleep(&self, today: NaiveDate, weekday: u8, minute: u32) {
        #[cfg(windows)]
        {
            static SLEPT: Mutex<Option<NaiveDate>> = Mutex::new(None);
            let schedule = self
                .app
                .state::<AppState>()
                .settings
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .power
                .clone();
            if schedule.sleep_at.is_none() {
                return;
            }
            let idle_min = (crate::system::screentime::probe::idle_ms() / 60_000) as u32;
            let slept = *SLEPT.lock().unwrap_or_else(|e| e.into_inner()) == Some(today);
            if crate::powerplan::sleep_due(&schedule, weekday, minute, idle_min, slept) {
                *SLEPT.lock().unwrap_or_else(|e| e.into_inner()) = Some(today);
                tracing::info!("power schedule: going to sleep");
                let _ = crate::commands::system::system_power(crate::commands::system::PowerAction::Sleep);
            }
        }
        #[cfg(not(windows))]
        let _ = (today, weekday, minute);
    }

    /// Keep "last session" up to date. A sudden drop to fewer than half the windows (apps
    /// closing as Windows shuts down) doesn't overwrite a fuller recent snapshot.
    pub fn start_session_keeper(&self) {
        #[cfg(windows)]
        {
            let this = self.clone();
            std::thread::Builder::new()
                .name("last-session".into())
                .spawn(move || {
                    // Not right at startup: that would save the empty desktop after a reboot.
                    std::thread::sleep(SESSION_EVERY);
                    loop {
                        this.save_last_session();
                        std::thread::sleep(SESSION_EVERY);
                    }
                })
                .ok();
        }
    }

    #[cfg(windows)]
    fn save_last_session(&self) {
        let windows = super::workspace::win::capture();
        if windows.is_empty() {
            return;
        }
        let now = Local::now().timestamp();
        let db = &self.app.state::<AppState>().db;
        let _ = db.with(|conn| {
            if let Some(prev) = super::workspace::get(conn, super::workspace::LAST_SESSION)? {
                let shrank = windows.len() * 2 < prev.windows.len();
                if shrank && now - prev.saved_at < 30 * 60 {
                    return Ok(());
                }
            }
            super::workspace::save(
                conn,
                &super::place::Workspace {
                    name: super::workspace::LAST_SESSION.into(),
                    saved_at: now,
                    windows,
                },
            )
        });
    }

    /// Snap a window into a zone of its screen and bring it forward.
    pub fn snap(&self, id: i64, zone: super::place::Zone) -> AppResult<()> {
        #[cfg(windows)]
        {
            let h = super::place::win::hwnd(id);
            let work = super::place::win::work_area(h)
                .ok_or_else(|| AppError::Platform("couldn't find that window's screen".into()))?;
            super::place::win::place(h, super::place::zone_rect(work, zone))?;
            let _ = super::win::focus(id);
            Ok(())
        }
        #[cfg(not(windows))]
        {
            let _ = (id, zone);
            Ok(())
        }
    }

    pub fn workspaces(&self) -> AppResult<Vec<super::place::Workspace>> {
        self.app.state::<AppState>().db.with(super::workspace::list)
    }

    /// Save every window's place under `name` (replacing one with the same name).
    pub fn workspace_save(&self, name: &str) -> AppResult<usize> {
        #[cfg(windows)]
        {
            let windows = super::workspace::win::capture();
            let count = windows.len();
            let ws = super::place::Workspace {
                name: name.to_string(),
                saved_at: Local::now().timestamp(),
                windows,
            };
            self.app
                .state::<AppState>()
                .db
                .with(|conn| super::workspace::save(conn, &ws))?;
            Ok(count)
        }
        #[cfg(not(windows))]
        {
            let _ = name;
            Ok(0)
        }
    }

    pub fn workspace_delete(&self, name: &str) -> AppResult<()> {
        self.app
            .state::<AppState>()
            .db
            .with(|conn| super::workspace::delete(conn, name))
    }

    /// Put a workspace back: (windows placed, programs started).
    pub fn workspace_restore(&self, name: &str) -> AppResult<(usize, usize)> {
        let ws = self
            .app
            .state::<AppState>()
            .db
            .with(|conn| super::workspace::get(conn, name))?
            .ok_or_else(|| AppError::InvalidInput("That workspace is gone".into()))?;
        #[cfg(windows)]
        return Ok(super::workspace::win::restore(&ws.windows));
        #[cfg(not(windows))]
        {
            let _ = ws;
            Ok((0, 0))
        }
    }

    /// After a restart (Overline started with Windows): the apps from last session that aren't
    /// open, once per run. Empty when there's nothing to offer.
    pub fn resume_offer(&self) -> AppResult<Vec<String>> {
        {
            let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
            if inner.offered {
                return Ok(Vec::new());
            }
            inner.offered = true;
        }
        if !std::env::args().any(|a| a == crate::shell::AUTOSTART_ARG) {
            return Ok(Vec::new());
        }
        let Some(last) = self
            .app
            .state::<AppState>()
            .db
            .with(|conn| super::workspace::get(conn, super::workspace::LAST_SESSION))?
        else {
            return Ok(Vec::new());
        };
        if Local::now().timestamp() - last.saved_at > RESUME_WITHIN_SECS {
            return Ok(Vec::new());
        }
        #[cfg(windows)]
        {
            let running = super::workspace::win::running_exes();
            Ok(super::workspace::missing(&last.windows, &running)
                .into_iter()
                .map(|w| crate::clipboard::app_label(&w.exe))
                .collect())
        }
        #[cfg(not(windows))]
        Ok(Vec::new())
    }

    /// The window switcher: one live view per tile (physical pixels, island window).
    pub fn grid_show(&self, island: &WebviewWindow, tiles: &[GridTile]) -> AppResult<()> {
        self.grid_hide(island);
        #[cfg(windows)]
        {
            let dest = windows::Win32::Foundation::HWND(island.hwnd()?.0);
            let mut ids = Vec::new();
            for (source, rect) in tiles.iter().take(12) {
                if let Ok(thumb) = super::win::thumbnail(dest, *source, *rect) {
                    ids.push(thumb);
                }
            }
            self.inner
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .grid
                .insert(island.label().to_owned(), ids);
        }
        #[cfg(not(windows))]
        let _ = tiles;
        Ok(())
    }

    pub fn grid_hide(&self, island: &WebviewWindow) {
        let removed = self
            .inner
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .grid
            .remove(island.label());
        #[cfg(windows)]
        for thumb in removed.into_iter().flatten() {
            super::win::drop_thumbnail(thumb);
        }
        #[cfg(not(windows))]
        let _ = removed;
    }

    pub fn windows(&self) -> Vec<DeskWindow> {
        #[cfg(windows)]
        return super::win::list();
        #[cfg(not(windows))]
        Vec::new()
    }

    pub fn set_on_top(&self, id: i64, on: bool) -> AppResult<()> {
        #[cfg(windows)]
        return super::win::set_on_top(id, on);
        #[cfg(not(windows))]
        {
            let _ = (id, on);
            Ok(())
        }
    }

    pub fn set_opacity(&self, id: i64, percent: u8) -> AppResult<()> {
        #[cfg(windows)]
        return super::win::set_opacity(id, percent);
        #[cfg(not(windows))]
        {
            let _ = (id, percent);
            Ok(())
        }
    }

    pub fn focus(&self, id: i64) -> AppResult<()> {
        #[cfg(windows)]
        return super::win::focus(id);
        #[cfg(not(windows))]
        {
            let _ = id;
            Ok(())
        }
    }

    /// The shortcut: pin the window in front on top, or unpin it.
    pub fn toggle_front_on_top(&self) {
        #[cfg(windows)]
        {
            let id = super::win::front();
            let Some(window) = super::win::list().into_iter().find(|w| w.id == id) else {
                return;
            };
            let on = !window.on_top;
            let result = super::win::set_on_top(id, on);
            self.emit(AppEvent::WindowPinned {
                title: window.title,
                on,
                ok: result.is_ok(),
            });
        }
    }

    /// Show a live view of `source` in this island at `rect` (physical pixels, island window).
    pub fn mini_show(
        &self,
        island: &WebviewWindow,
        source: i64,
        rect: (i32, i32, i32, i32),
    ) -> AppResult<()> {
        #[cfg(windows)]
        {
            let label = island.label().to_owned();
            let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
            if let Some((thumb, current)) = inner.thumbnails.get(&label).copied() {
                if current == source {
                    return super::win::move_thumbnail(thumb, rect);
                }
                super::win::drop_thumbnail(thumb);
                inner.thumbnails.remove(&label);
            }
            let dest = windows::Win32::Foundation::HWND(island.hwnd()?.0);
            let thumb = super::win::thumbnail(dest, source, rect)?;
            inner.thumbnails.insert(label, (thumb, source));
            Ok(())
        }
        #[cfg(not(windows))]
        {
            let _ = (island, source, rect);
            Ok(())
        }
    }

    pub fn mini_hide(&self, island: &WebviewWindow) {
        let removed = self
            .inner
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .thumbnails
            .remove(island.label());
        #[cfg(windows)]
        if let Some((thumb, _)) = removed {
            super::win::drop_thumbnail(thumb);
        }
        #[cfg(not(windows))]
        let _ = removed;
    }

    /// Run a routine's steps in order. `scheduled`: started by its schedule (the card says so).
    pub fn run(&self, id: &str, scheduled: bool) -> AppResult<usize> {
        use tauri_plugin_opener::OpenerExt;
        let routine = self
            .routines()
            .into_iter()
            .find(|r| r.id == id)
            .ok_or_else(|| AppError::InvalidInput("That routine is gone".into()))?;
        let mut done = 0;
        let mut failed = 0;
        for step in &routine.steps {
            let ok = match step {
                RoutineStep::App { path } | RoutineStep::Folder { path } => {
                    self.app.opener().open_path(path, None::<&str>).is_ok()
                }
                RoutineStep::Url { url } => self.app.opener().open_url(url, None::<&str>).is_ok(),
                RoutineStep::Close { exe } => {
                    #[cfg(windows)]
                    super::win::close_app(&exe.to_lowercase());
                    #[cfg(not(windows))]
                    let _ = exe;
                    true
                }
                RoutineStep::Focus { minutes } => self
                    .app
                    .state::<crate::focus::service::FocusService>()
                    .start(*minutes)
                    .is_ok(),
            };
            if ok {
                done += 1;
            } else {
                failed += 1;
            }
            std::thread::sleep(STEP_GAP);
        }
        tracing::info!(steps = done, failed, scheduled, "routine ran");
        self.emit(AppEvent::RoutineRan {
            name: routine.name,
            done: done as u32,
            failed: failed as u32,
            scheduled,
        });
        Ok(done)
    }
}
