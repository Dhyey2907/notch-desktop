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

const SCHEDULE_TICK: Duration = Duration::from_secs(30);
/// A short pause between steps, so apps don't all start in the same instant.
const STEP_GAP: Duration = Duration::from_millis(350);

#[derive(Default)]
struct Inner {
    /// Island label → (thumbnail id, source window).
    thumbnails: HashMap<String, (isize, i64)>,
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
