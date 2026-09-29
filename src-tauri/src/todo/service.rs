//! Owns the timer scheduler, the "finished, not yet acknowledged" set, and the ring.

use std::{
    collections::HashSet,
    sync::{Arc, Mutex},
    time::Duration as StdDuration,
};

use chrono::{DateTime, Duration, Utc};
use tauri::{AppHandle, Manager};
use tokio::sync::Notify;

use super::{Note, Task, TodoSnapshot};
use crate::{error::AppResult, events::AppEvent, state::AppState};

/// A timer noticed this long after it was due finished while Notch wasn't running.
const LATE_AFTER: Duration = Duration::seconds(90);
/// The ring repeats this often, for at most `RING_FOR`, until acknowledged.
const RING_EVERY: StdDuration = StdDuration::from_secs(4);
const RING_FOR: StdDuration = StdDuration::from_secs(60);
/// Longest sleep between scheduler checks (guards against clock changes).
const MAX_SLEEP: StdDuration = StdDuration::from_secs(3600);

struct Inner {
    app: AppHandle,
    wake: Notify,
    /// Finished timers waiting for Done / +5 min / Dismiss.
    alerting: Mutex<Vec<String>>,
    /// Subset of `alerting` that is still ringing.
    ringing: Mutex<HashSet<String>>,
}

#[derive(Clone)]
pub struct TodoService {
    inner: Arc<Inner>,
}

impl TodoService {
    pub fn new(app: AppHandle) -> Self {
        Self {
            inner: Arc::new(Inner {
                app,
                wake: Notify::new(),
                alerting: Mutex::new(Vec::new()),
                ringing: Mutex::new(HashSet::new()),
            }),
        }
    }

    pub fn start(&self) {
        let inner = self.inner.clone();
        tauri::async_runtime::spawn(async move { inner.run().await });
    }

    pub fn snapshot(&self) -> AppResult<TodoSnapshot> {
        self.inner.snapshot()
    }

    /// Run a change, then refresh every island and re-arm the scheduler.
    fn change<T>(
        &self,
        f: impl FnOnce(&rusqlite::Connection, DateTime<Utc>) -> AppResult<T>,
    ) -> AppResult<T> {
        let now = Utc::now();
        let result = self.inner.db(|conn| f(conn, now))?;
        self.inner.publish();
        self.inner.wake.notify_one();
        Ok(result)
    }

    pub fn add_task(&self, title: &str, timer_minutes: Option<u32>, ring: bool) -> AppResult<Task> {
        self.change(|conn, now| super::add_task(conn, title, timer_minutes, ring, now))
    }

    pub fn set_done(&self, id: &str, done: bool) -> AppResult<()> {
        self.inner.acknowledge(id);
        self.change(|conn, now| super::set_done(conn, id, done, now))
    }

    pub fn rename(&self, id: &str, title: &str) -> AppResult<()> {
        self.change(|conn, _| super::rename(conn, id, title))
    }

    pub fn delete_task(&self, id: &str) -> AppResult<()> {
        self.inner.acknowledge(id);
        self.change(|conn, _| super::delete_task(conn, id))
    }

    pub fn clear_done(&self) -> AppResult<()> {
        self.change(|conn, _| super::clear_done(conn).map(|_| ()))
    }

    /// Start (or restart, e.g. "+5 min") a task's timer.
    pub fn start_timer(&self, id: &str, minutes: u32, ring: bool) -> AppResult<()> {
        self.inner.acknowledge(id);
        self.change(|conn, now| super::start_timer(conn, id, minutes, ring, now))
    }

    pub fn cancel_timer(&self, id: &str) -> AppResult<()> {
        self.inner.acknowledge(id);
        self.change(|conn, _| super::cancel_timer(conn, id))
    }

    /// Dismiss a finished timer: stop ringing and clear the alert on every screen.
    pub fn acknowledge(&self, id: &str) {
        if self.inner.acknowledge(id) {
            self.inner.publish();
        }
    }

    pub fn add_note(&self, body: &str) -> AppResult<Note> {
        self.change(|conn, now| super::add_note(conn, body, now))
    }

    pub fn update_note(&self, id: &str, body: &str) -> AppResult<()> {
        self.change(|conn, now| super::update_note(conn, id, body, now))
    }

    pub fn delete_note(&self, id: &str) -> AppResult<()> {
        self.change(|conn, _| super::delete_note(conn, id))
    }
}

impl Inner {
    fn db<T>(&self, f: impl FnOnce(&rusqlite::Connection) -> AppResult<T>) -> AppResult<T> {
        self.app.state::<AppState>().db.with(f)
    }

    fn snapshot(&self) -> AppResult<TodoSnapshot> {
        let (tasks, notes) = self.db(|conn| Ok((super::list_tasks(conn)?, super::list_notes(conn)?)))?;
        let alerting = self.alerting.lock().unwrap_or_else(|e| e.into_inner()).clone();
        Ok(TodoSnapshot {
            tasks,
            notes,
            alerting,
        })
    }

    fn publish(&self) {
        match self.snapshot() {
            Ok(snapshot) => self
                .app
                .state::<AppState>()
                .bus
                .emit("todo", AppEvent::TodoChanged(snapshot)),
            Err(error) => tracing::warn!(kind = error.kind(), "could not read to-do list"),
        }
    }

    /// Returns true if the task was alerting.
    fn acknowledge(&self, id: &str) -> bool {
        self.ringing.lock().unwrap_or_else(|e| e.into_inner()).remove(id);
        let mut alerting = self.alerting.lock().unwrap_or_else(|e| e.into_inner());
        let before = alerting.len();
        alerting.retain(|a| a != id);
        alerting.len() != before
    }

    async fn run(self: Arc<Self>) {
        loop {
            let now = Utc::now();
            let due = self
                .app
                .state::<AppState>()
                .db
                .with_mut(|conn| super::take_due_timers(conn, now));
            match due {
                Ok(due) if !due.is_empty() => {
                    for task in due {
                        self.finished(task, now);
                    }
                    self.publish();
                }
                Ok(_) => {}
                Err(error) => tracing::warn!(kind = error.kind(), "could not check to-do timers"),
            }

            let next = self.db(super::next_timer_at).ok().flatten();
            let sleep = next
                .map(|at| (at - Utc::now()).to_std().unwrap_or_default())
                .unwrap_or(MAX_SLEEP)
                .min(MAX_SLEEP);
            tokio::select! {
                _ = tokio::time::sleep(sleep) => {}
                _ = self.wake.notified() => {}
            }
        }
    }

    fn finished(self: &Arc<Self>, task: Task, now: DateTime<Utc>) {
        let late = task.timer_ends_at.is_some_and(|due| now - due > LATE_AFTER);
        {
            let mut alerting = self.alerting.lock().unwrap_or_else(|e| e.into_inner());
            if !alerting.contains(&task.id) {
                alerting.push(task.id.clone());
            }
        }
        // Don't ring for timers that ran out while Notch was closed.
        if task.ring && !late {
            self.ring(task.id.clone());
        }
        tracing::debug!(late, ring = task.ring, "to-do timer finished");
        self.app
            .state::<AppState>()
            .bus
            .emit("todo", AppEvent::TimerDone { task, late });
    }

    /// Ring until acknowledged, for at most a minute.
    fn ring(self: &Arc<Self>, id: String) {
        self.ringing
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .insert(id.clone());
        let inner = self.clone();
        tauri::async_runtime::spawn(async move {
            let started = tokio::time::Instant::now();
            while started.elapsed() < RING_FOR
                && inner
                    .ringing
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .contains(&id)
            {
                play_chime();
                tokio::time::sleep(RING_EVERY).await;
            }
            inner
                .ringing
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .remove(&id);
        });
    }
}

/// Windows' own "Reminder" sound, following the user's sound scheme and volume.
#[cfg(windows)]
fn play_chime() {
    use windows::{
        core::w,
        Win32::Media::Audio::{PlaySoundW, SND_ALIAS, SND_ASYNC, SND_NODEFAULT},
    };
    // SAFETY: plays a registered system sound asynchronously; the alias is a static string.
    unsafe {
        let _ = PlaySoundW(
            w!("Notification.Reminder"),
            None,
            SND_ALIAS | SND_ASYNC | SND_NODEFAULT,
        );
    }
}

#[cfg(not(windows))]
fn play_chime() {}
