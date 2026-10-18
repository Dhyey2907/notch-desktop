//! Shelf changes go through here so every island hears about them.

use chrono::Utc;
use tauri::{AppHandle, Manager, WebviewWindow};

use super::ShelfSnapshot;
use crate::{
    error::{AppError, AppResult},
    events::AppEvent,
    state::AppState,
};

#[derive(Clone)]
pub struct ShelfService {
    app: AppHandle,
}

impl ShelfService {
    pub fn new(app: AppHandle) -> Self {
        Self { app }
    }

    fn db<T>(&self, f: impl FnOnce(&rusqlite::Connection) -> AppResult<T>) -> AppResult<T> {
        self.app.state::<AppState>().db.with(f)
    }

    /// Re-reads each file, so moved or deleted ones show as missing. Old unpinned files are
    /// cleaned off first.
    pub fn snapshot(&self) -> AppResult<ShelfSnapshot> {
        let keep = super::keep_for(
            &self
                .app
                .state::<AppState>()
                .settings
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .clipboard
                .shelf_keep,
        );
        self.db(|conn| super::prune(conn, Utc::now(), keep))?;
        self.db(super::list)
    }

    /// Clean the shelf now and then, even while nobody looks at it.
    pub fn start(&self) {
        let this = self.clone();
        super::screenshots::watch(move |path| {
            let state = this.app.state::<AppState>();
            if state
                .settings
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .clipboard
                .no_screenshots
            {
                return;
            }
            let text = path.display().to_string();
            let _ = this.add(std::slice::from_ref(&text));
            let name = path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default();
            state
                .bus
                .emit("shelf", AppEvent::ScreenshotTaken { path: text, name });
        });
        let this = self.clone();
        std::thread::spawn(move || loop {
            std::thread::sleep(std::time::Duration::from_secs(3600));
            this.publish();
        });
    }

    pub fn pin(&self, id: &str, pinned: bool) -> AppResult<()> {
        self.db(|conn| super::set_pinned(conn, id, pinned))?;
        self.publish();
        Ok(())
    }

    fn publish(&self) {
        match self.snapshot() {
            Ok(snapshot) => self
                .app
                .state::<AppState>()
                .bus
                .emit("shelf", AppEvent::ShelfChanged(snapshot)),
            Err(error) => tracing::warn!(kind = error.kind(), "could not read the shelf"),
        }
    }

    pub fn add(&self, paths: &[String]) -> AppResult<usize> {
        let added = self.db(|conn| super::add(conn, paths, Utc::now()))?;
        self.publish();
        Ok(added)
    }

    pub fn remove(&self, ids: &[String]) -> AppResult<()> {
        self.db(|conn| super::remove(conn, ids))?;
        self.publish();
        Ok(())
    }

    pub fn clear(&self) -> AppResult<()> {
        self.db(super::clear)?;
        self.publish();
        Ok(())
    }

    /// Put the files on the clipboard, to paste anywhere (empty `ids`: everything).
    pub fn copy(&self, ids: &[String]) -> AppResult<()> {
        let paths = self.db(|conn| super::existing_paths(conn, ids))?;
        #[cfg(windows)]
        {
            let files: Vec<String> = paths.iter().map(|p| p.display().to_string()).collect();
            crate::clipboard::win::write(crate::clipboard::win::Put::Files(&files))
        }
        #[cfg(not(windows))]
        {
            let _ = paths;
            Err(AppError::Platform("not supported".into()))
        }
    }

    pub fn reveal(&self, id: &str) -> AppResult<()> {
        let path = self
            .db(|conn| super::existing_paths(conn, &[id.to_owned()]))?
            .remove(0);
        #[cfg(windows)]
        return super::win::reveal(&path);
        #[cfg(not(windows))]
        {
            let _ = path;
            Err(AppError::Platform("not supported".into()))
        }
    }

    /// Drag files out of `window` (empty `ids`: everything). Resolves when dropped (true)
    /// or cancelled (false).
    pub async fn drag(&self, window: WebviewWindow, ids: Vec<String>) -> AppResult<bool> {
        let paths = self.db(|conn| super::existing_paths(conn, &ids))?;
        #[cfg(windows)]
        {
            let hwnd = window.hwnd()?.0 as isize;
            let (tx, rx) = tokio::sync::oneshot::channel();
            // OLE drag and drop runs on the UI thread, which owns the window.
            window.run_on_main_thread(move || {
                let hwnd = windows::Win32::Foundation::HWND(hwnd as *mut _);
                let _ = tx.send(super::win::drag_files(hwnd, &paths));
            })?;
            rx.await
                .map_err(|_| AppError::Platform("drag was interrupted".into()))?
        }
        #[cfg(not(windows))]
        {
            let _ = (window, paths);
            Err(AppError::Platform("not supported".into()))
        }
    }
}
