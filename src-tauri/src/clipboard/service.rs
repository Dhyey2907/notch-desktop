//! Owns clipboard history: takes each copy from the listener, applies the privacy rules,
//! stores pictures as PNGs in the cache, and tells every island when the list changes.

use std::{
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use chrono::Utc;
use tauri::{AppHandle, Manager};

use super::{ClipboardSnapshot, Content, Entry, History};
use crate::{
    error::{AppError, AppResult},
    events::AppEvent,
    state::AppState,
};

struct Inner {
    app: AppHandle,
    history: Mutex<History>,
    /// PNGs of copied pictures.
    dir: PathBuf,
}

#[derive(Clone)]
pub struct ClipboardService {
    inner: Arc<Inner>,
}

impl ClipboardService {
    pub fn new(app: AppHandle, dir: PathBuf) -> Self {
        Self {
            inner: Arc::new(Inner {
                app,
                history: Mutex::new(History::default()),
                dir,
            }),
        }
    }

    pub fn start(&self) {
        let pins = self.inner.db(super::load_pins).unwrap_or_else(|error| {
            tracing::warn!(kind = error.kind(), "could not read pinned clips");
            Vec::new()
        });
        *self.inner.history() = History::with_pinned(pins);
        self.inner.remove_unused_images();
        #[cfg(windows)]
        {
            let inner = self.inner.clone();
            super::win::listen(move |captured| inner.on_copy(captured));
        }
    }

    pub fn snapshot(&self) -> ClipboardSnapshot {
        self.inner.history().snapshot()
    }

    /// Put a clip back on the clipboard (`formatted`: JSON indented) and move it to the top.
    pub fn copy(&self, id: &str, formatted: bool) -> AppResult<()> {
        let content = self.inner.content(id)?;
        #[cfg(windows)]
        {
            use super::win::{write, Put};
            match &content {
                Content::Text(text) => {
                    let pretty = formatted.then(|| super::pretty_json(text)).flatten();
                    write(Put::Text(pretty.as_deref().unwrap_or(text)))?;
                }
                Content::Image(path) => write(Put::Image(&std::fs::read(path)?))?,
                Content::Files(files) => write(Put::Files(files))?,
            }
        }
        #[cfg(not(windows))]
        let _ = (content, formatted);
        self.inner.history().bump(id, Utc::now());
        self.inner.publish();
        Ok(())
    }

    /// The whole text of a clip (the list only carries the start of it).
    pub fn text(&self, id: &str) -> AppResult<String> {
        match self.inner.content(id)? {
            Content::Text(text) => Ok(text),
            _ => Err(AppError::InvalidInput("That clip isn't text".into())),
        }
    }

    pub fn pin(&self, id: &str) -> AppResult<()> {
        let entry = self.inner.history().pin(id)?;
        if let Err(error) = self.inner.db(|conn| super::save_pin(conn, &entry, Utc::now())) {
            let _ = self.inner.history().unpin(id);
            return Err(error);
        }
        self.inner.publish();
        Ok(())
    }

    pub fn unpin(&self, id: &str) -> AppResult<()> {
        self.inner.history().unpin(id)?;
        self.inner.db(|conn| super::delete_pin(conn, id))?;
        self.inner.publish();
        Ok(())
    }

    pub fn delete(&self, id: &str) -> AppResult<()> {
        let removed = self
            .inner
            .history()
            .delete(id)
            .ok_or_else(|| AppError::InvalidInput("That clip is no longer in history".into()))?;
        if removed.clip.pinned {
            self.inner.db(|conn| super::delete_pin(conn, id))?;
        }
        self.inner.forget(vec![removed]);
        self.inner.publish();
        Ok(())
    }

    /// Empty history (pinned clips stay).
    pub fn clear(&self) {
        let removed = self.inner.history().clear();
        self.inner.forget(removed);
        self.inner.publish();
    }
}

impl Inner {
    fn history(&self) -> std::sync::MutexGuard<'_, History> {
        self.history.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn db<T>(&self, f: impl FnOnce(&rusqlite::Connection) -> AppResult<T>) -> AppResult<T> {
        self.app.state::<AppState>().db.with(f)
    }

    fn content(&self, id: &str) -> AppResult<Content> {
        self.history()
            .get(id)
            .map(|e| e.content.clone())
            .ok_or_else(|| AppError::InvalidInput("That clip is no longer in history".into()))
    }

    fn publish(&self) {
        let snapshot = self.history().snapshot();
        self.app
            .state::<AppState>()
            .bus
            .emit("clipboard", AppEvent::ClipboardChanged(snapshot));
    }

    #[cfg(windows)]
    fn on_copy(&self, captured: super::win::Captured) {
        use super::win::Raw;
        // Notch re-copying a clip from the list: already moved to the top.
        if captured.own {
            return;
        }
        let (paused, blocked) = {
            let settings = self
                .app
                .state::<AppState>()
                .settings
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .clipboard
                .clone();
            (settings.paused, settings.blocked_apps)
        };
        if paused {
            return;
        }
        if captured.private || super::is_blocked(captured.exe.as_deref(), &blocked) {
            // Only that something was skipped; never what.
            tracing::debug!("skipped a private copy");
            return;
        }
        let app = captured.exe.as_deref().map(super::app_label);
        let now = Utc::now();
        let entry = match captured.raw {
            Some(Raw::Text(text)) => super::text_entry(text, app, now),
            Some(Raw::Files(files)) => super::files_entry(files, app, now),
            Some(Raw::Png(png)) => super::image::png_size(&png)
                .filter(|(w, h)| u64::from(*w) * u64::from(*h) <= super::image::MAX_PIXELS)
                .and_then(|size| self.picture(png, size, app, now)),
            Some(Raw::Dib(dib)) => {
                super::image::dib_to_png(&dib).and_then(|(png, w, h)| self.picture(png, (w, h), app, now))
            }
            None => None,
        };
        let Some(entry) = entry else { return };
        let evicted = self.history().add(entry);
        self.forget(evicted);
        self.publish();
    }

    /// Store a copied picture under its content hash (the same picture is stored once).
    fn picture(
        &self,
        png: Vec<u8>,
        size: (u32, u32),
        app: Option<String>,
        now: chrono::DateTime<Utc>,
    ) -> Option<Entry> {
        let hash = super::hash_of(super::ClipKind::Image, &png);
        let path = self.dir.join(format!("{hash:016x}.png"));
        if !path.exists() {
            let written = std::fs::create_dir_all(&self.dir).and_then(|()| std::fs::write(&path, &png));
            if let Err(error) = written {
                tracing::warn!(%error, "could not save a copied picture");
                return None;
            }
        }
        Some(super::image_entry(path, hash, size, app, now))
    }

    /// Delete pictures no clip shows any more.
    fn forget(&self, removed: Vec<Entry>) {
        let history = self.history();
        for entry in removed {
            if let Content::Image(path) = &entry.content {
                if !history.uses_image(path) {
                    let _ = std::fs::remove_file(path);
                }
            }
        }
    }

    /// History starts empty on launch: keep only pinned pictures.
    fn remove_unused_images(&self) {
        let Ok(files) = std::fs::read_dir(&self.dir) else {
            return;
        };
        let history = self.history();
        for file in files.flatten() {
            let path = file.path();
            if is_clip_png(&path) && !history.uses_image(&path) {
                let _ = std::fs::remove_file(&path);
            }
        }
    }
}

fn is_clip_png(path: &Path) -> bool {
    path.extension().is_some_and(|e| e == "png")
}
