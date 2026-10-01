//! Owns the focus timer: sleeps until the session ends (or is changed), then announces it.

use std::{sync::Arc, time::Duration as StdDuration};

use chrono::Utc;
use tauri::{AppHandle, Manager};
use tokio::sync::Notify;

use super::FocusState;
use crate::{error::AppResult, events::AppEvent, state::AppState};

#[derive(Clone)]
pub struct FocusService {
    app: AppHandle,
    wake: Arc<Notify>,
}

impl FocusService {
    pub fn new(app: AppHandle) -> Self {
        Self {
            app,
            wake: Arc::default(),
        }
    }

    fn db<T>(&self, f: impl FnOnce(&rusqlite::Connection) -> AppResult<T>) -> AppResult<T> {
        self.app.state::<AppState>().db.with(f)
    }

    pub fn state(&self) -> AppResult<FocusState> {
        self.db(|conn| super::state(conn, Utc::now()))
    }

    fn publish(&self) {
        match self.state() {
            Ok(state) => self
                .app
                .state::<AppState>()
                .bus
                .emit("focus", AppEvent::FocusChanged(state)),
            Err(error) => tracing::warn!(kind = error.kind(), "could not read focus state"),
        }
    }

    pub fn start(&self, minutes: u32) -> AppResult<()> {
        self.db(|conn| super::start(conn, minutes, Utc::now()))?;
        self.publish();
        self.wake.notify_one();
        Ok(())
    }

    pub fn stop(&self) -> AppResult<()> {
        self.db(super::stop)?;
        self.publish();
        self.wake.notify_one();
        Ok(())
    }

    pub fn run(&self) {
        let this = self.clone();
        tauri::async_runtime::spawn(async move {
            loop {
                let now = Utc::now();
                match this.db(|conn| super::finish_if_due(conn, now)) {
                    Ok(true) => {
                        this.publish();
                        this.app
                            .state::<AppState>()
                            .bus
                            .emit("focus", AppEvent::FocusDone);
                    }
                    Ok(false) => {}
                    Err(error) => tracing::warn!(kind = error.kind(), "could not check focus"),
                }
                let sleep = this
                    .db(super::current)
                    .ok()
                    .flatten()
                    .map(|(_, ends)| (ends - Utc::now()).to_std().unwrap_or_default())
                    .unwrap_or(StdDuration::from_secs(3600))
                    .min(StdDuration::from_secs(3600));
                tokio::select! {
                    _ = tokio::time::sleep(sleep) => {}
                    _ = this.wake.notified() => {}
                }
            }
        });
    }
}
