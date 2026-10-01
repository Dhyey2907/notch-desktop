//! Downloads, recent files, shelf tools and the recorder, for every island.

use std::{
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use tauri::{AppHandle, Manager, WebviewWindow};

use super::{convert::ImageTarget, DownloadProgress, RecentFile, RecorderState};
use crate::{
    error::{AppError, AppResult},
    events::AppEvent,
    shelf::service::ShelfService,
    state::AppState,
};

/// Recent files shown.
const RECENT: usize = 12;

#[derive(Default)]
struct Inner {
    downloads: Vec<DownloadProgress>,
    #[cfg(windows)]
    recording: Option<(super::recorder::Recording, i64)>,
}

#[derive(Clone)]
pub struct FilesService {
    app: AppHandle,
    inner: Arc<Mutex<Inner>>,
}

/// Only existing, absolute file paths without quotes come in from the UI.
fn check_file(path: &str) -> AppResult<PathBuf> {
    let p = Path::new(path);
    if !p.is_absolute() || path.contains('"') || path.len() > 2000 || !p.is_file() {
        return Err(AppError::InvalidInput("That file doesn't exist any more".into()));
    }
    Ok(p.to_path_buf())
}

impl FilesService {
    pub fn new(app: AppHandle) -> Self {
        Self {
            app,
            inner: Arc::default(),
        }
    }

    fn emit(&self, event: AppEvent) {
        self.app.state::<AppState>().bus.emit("files", event);
    }

    pub fn start(&self) {
        #[cfg(windows)]
        {
            let Ok(dir) = self.app.path().download_dir() else {
                return;
            };
            let progress = self.clone();
            let done = self.clone();
            super::downloads::watch(
                dir,
                move |list| {
                    progress.inner.lock().unwrap_or_else(|e| e.into_inner()).downloads = list.clone();
                    progress.emit(AppEvent::DownloadsChanged(list));
                },
                move |finished| done.emit(AppEvent::DownloadDone(finished)),
            );
        }
    }

    pub fn downloads(&self) -> Vec<DownloadProgress> {
        self.inner
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .downloads
            .clone()
    }

    pub fn recent(&self) -> Vec<RecentFile> {
        #[cfg(windows)]
        return super::recent::read(RECENT);
        #[cfg(not(windows))]
        Vec::new()
    }

    /// Run a shelf tool, then put the new file on the shelf.
    fn made(&self, path: PathBuf) -> AppResult<String> {
        let text = path.to_string_lossy().into_owned();
        self.app
            .state::<ShelfService>()
            .add(std::slice::from_ref(&text))?;
        Ok(text)
    }

    pub fn convert(&self, path: &str, to: ImageTarget) -> AppResult<String> {
        self.made(super::convert::convert_image(&check_file(path)?, to)?)
    }

    pub fn shrink(&self, path: &str) -> AppResult<String> {
        self.made(super::convert::shrink_image(&check_file(path)?)?)
    }

    pub fn merge(&self, paths: &[String]) -> AppResult<String> {
        let paths = paths
            .iter()
            .map(|p| check_file(p))
            .collect::<AppResult<Vec<_>>>()?;
        self.made(super::convert::merge_pdfs(&paths)?)
    }

    pub fn open(&self, path: &str) -> AppResult<()> {
        use tauri_plugin_opener::OpenerExt;
        let path = check_file(path)?;
        self.app
            .opener()
            .open_path(path.to_string_lossy(), None::<&str>)
            .map_err(|e| AppError::Platform(e.to_string()))
    }

    pub fn reveal(&self, path: &str) -> AppResult<()> {
        let path = check_file(path)?;
        #[cfg(windows)]
        return crate::shelf::reveal(&path);
        #[cfg(not(windows))]
        {
            let _ = path;
            Err(AppError::Platform("not supported".into()))
        }
    }

    /// Drag files (e.g. recent ones) out of `window` into another app.
    pub async fn drag(&self, window: WebviewWindow, paths: Vec<String>) -> AppResult<bool> {
        let paths = paths
            .iter()
            .map(|p| check_file(p))
            .collect::<AppResult<Vec<_>>>()?;
        #[cfg(windows)]
        {
            let hwnd = window.hwnd()?.0 as isize;
            let (tx, rx) = tokio::sync::oneshot::channel();
            window.run_on_main_thread(move || {
                let hwnd = windows::Win32::Foundation::HWND(hwnd as *mut _);
                let _ = tx.send(crate::shelf::drag_files(hwnd, &paths));
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

    pub fn recorder(&self) -> RecorderState {
        #[cfg(windows)]
        {
            let inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
            RecorderState {
                recording: inner.recording.is_some(),
                started_at: inner.recording.as_ref().map(|(_, at)| *at),
            }
        }
        #[cfg(not(windows))]
        RecorderState::default()
    }

    pub fn record_start(&self) -> AppResult<RecorderState> {
        #[cfg(windows)]
        {
            let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
            if inner.recording.is_none() {
                let dir = self
                    .app
                    .path()
                    .audio_dir()
                    .map_err(|e| AppError::Platform(e.to_string()))?
                    .join("Notch recordings");
                std::fs::create_dir_all(&dir)?;
                let name = super::recording_name(chrono::Local::now(), "wav");
                let wav = crate::bridge::unique_path(&dir, &name);
                let started = chrono::Utc::now().timestamp_millis();
                inner.recording = Some((super::recorder::start(wav), started));
                tracing::info!("recording started");
                // Stop on its own at the length limit.
                let this = self.clone();
                std::thread::spawn(move || {
                    std::thread::sleep(super::recorder::MAX_LENGTH);
                    if this.recorder().started_at == Some(started) {
                        let _ = this.record_stop();
                    }
                });
            }
        }
        let state = self.recorder();
        self.emit(AppEvent::RecorderChanged(state.clone()));
        Ok(state)
    }

    /// Finish the recording and put it on the shelf; returns its path.
    pub fn record_stop(&self) -> AppResult<Option<String>> {
        #[cfg(windows)]
        {
            let recording = self
                .inner
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .recording
                .take();
            self.emit(AppEvent::RecorderChanged(self.recorder()));
            let Some((recording, _)) = recording else {
                return Ok(None);
            };
            let path = recording.finish()?;
            tracing::info!("recording saved");
            let text = self.made(path.clone())?;
            self.emit(AppEvent::RecordingSaved {
                name: path
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default(),
                path: text.clone(),
            });
            Ok(Some(text))
        }
        #[cfg(not(windows))]
        Ok(None)
    }
}
