//! Snippets: stored in the database; the keyboard hook runs only while there's at least one
//! snippet and snippets aren't turned off.

use chrono::Utc;
use tauri::{AppHandle, Manager};

use super::{validate, Snippet};
use crate::{
    error::{AppError, AppResult},
    events::AppEvent,
    state::AppState,
};

#[derive(Clone)]
pub struct ExpanderService {
    app: AppHandle,
}

impl ExpanderService {
    pub fn new(app: AppHandle) -> Self {
        Self { app }
    }

    fn db<T>(&self, f: impl FnOnce(&rusqlite::Connection) -> AppResult<T>) -> AppResult<T> {
        self.app.state::<AppState>().db.with(f)
    }

    pub fn list(&self) -> AppResult<Vec<Snippet>> {
        self.db(|conn| {
            let mut stmt = conn.prepare("SELECT id, trigger, body FROM snippets ORDER BY trigger")?;
            let rows = stmt
                .query_map([], |r| {
                    Ok(Snippet {
                        id: r.get(0)?,
                        trigger: r.get(1)?,
                        body: r.get(2)?,
                    })
                })?
                .collect::<Result<Vec<_>, _>>()?;
            Ok(rows)
        })
    }

    /// Load snippets into the hook and switch it on or off to match the settings.
    pub fn apply(&self) {
        let snippets = self.list().unwrap_or_default();
        let off = self
            .app
            .state::<AppState>()
            .settings
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clipboard
            .no_snippets;
        #[cfg(windows)]
        {
            let active = !off && !snippets.is_empty();
            *super::hook::SNIPPETS.write().unwrap_or_else(|e| e.into_inner()) = snippets
                .iter()
                .map(|s| (s.trigger.clone(), s.body.clone()))
                .collect();
            super::hook::ENABLED.store(active, std::sync::atomic::Ordering::SeqCst);
            if active {
                super::hook::start();
            }
        }
        #[cfg(not(windows))]
        let _ = (snippets, off);
    }

    pub fn start(&self) {
        self.apply();
        let this = self.clone();
        let mut events = self.app.state::<AppState>().bus.subscribe();
        tauri::async_runtime::spawn(async move {
            let mut off = None;
            loop {
                match events.recv().await {
                    Ok(envelope) => {
                        if let AppEvent::SettingsChanged(s) = &envelope.event {
                            if off != Some(s.clipboard.no_snippets) {
                                off = Some(s.clipboard.no_snippets);
                                this.apply();
                            }
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                    Err(_) => break,
                }
            }
        });
    }

    fn changed(&self) -> AppResult<Vec<Snippet>> {
        self.apply();
        let list = self.list()?;
        self.app
            .state::<AppState>()
            .bus
            .emit("snippets", AppEvent::SnippetsChanged(list.clone()));
        Ok(list)
    }

    pub fn save(&self, id: Option<String>, trigger: &str, body: &str) -> AppResult<Vec<Snippet>> {
        let trigger = trigger.trim();
        validate(trigger, body)?;
        let taken = self
            .list()?
            .into_iter()
            .any(|s| s.trigger == trigger && Some(&s.id) != id.as_ref());
        if taken {
            return Err(AppError::InvalidInput(format!("{trigger} is already a trigger")));
        }
        self.db(|conn| {
            match &id {
                Some(id) => conn.execute(
                    "UPDATE snippets SET trigger = ?2, body = ?3 WHERE id = ?1",
                    rusqlite::params![id, trigger, body],
                )?,
                None => conn.execute(
                    "INSERT INTO snippets (id, trigger, body, created_at) VALUES (?1, ?2, ?3, ?4)",
                    rusqlite::params![
                        uuid::Uuid::new_v4().to_string(),
                        trigger,
                        body,
                        Utc::now().to_rfc3339()
                    ],
                )?,
            };
            Ok(())
        })?;
        self.changed()
    }

    pub fn delete(&self, id: &str) -> AppResult<Vec<Snippet>> {
        self.db(|conn| {
            conn.execute("DELETE FROM snippets WHERE id = ?1", [id])?;
            Ok(())
        })?;
        self.changed()
    }
}
