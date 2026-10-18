//! Wallpaper changes go through here so every island hears about them. Everything here does
//! file or COM work, so callers run it on a blocking thread.

use std::{
    collections::HashSet,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
};

use tauri::{AppHandle, Manager};

use super::{
    already_saved, check_picture, describe, desktop, make_thumb, pictures_in, push_recent, resolve_target,
    saved_name, thumb_key, Wallpaper, WallpaperState, ALL, MAX_BYTES, MAX_FOLDER, MAX_SAVED,
};
use crate::{
    error::{AppError, AppResult},
    events::AppEvent,
    settings::{Settings, WallpaperSettings},
    state::AppState,
};

const THUMB_HEIGHT: u32 = 360;

/// A wallpaper's thumbnail key and its colours (none if it couldn't be read).
type CachedAccent = (String, Option<crate::music::Palette>);

#[derive(Clone)]
pub struct WallpaperService {
    app: AppHandle,
    /// Overline's own copies of saved photos.
    saved_dir: PathBuf,
    thumbs_dir: PathBuf,
    making_thumbs: Arc<AtomicBool>,
    /// Pictures that couldn't be read, so they aren't retried on every refresh.
    unreadable: Arc<Mutex<HashSet<PathBuf>>>,
    /// The main screen's wallpaper colours, by picture.
    accent_cache: Arc<Mutex<Option<CachedAccent>>>,
}

impl WallpaperService {
    pub fn new(app: AppHandle, saved_dir: PathBuf, thumbs_dir: PathBuf) -> Self {
        Self {
            app,
            saved_dir,
            thumbs_dir,
            making_thumbs: Arc::new(AtomicBool::new(false)),
            unreadable: Arc::new(Mutex::new(HashSet::new())),
            accent_cache: Arc::default(),
        }
    }

    fn settings(&self) -> WallpaperSettings {
        self.app
            .state::<AppState>()
            .settings
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .wallpaper
            .clone()
    }

    fn update(&self, change: impl FnOnce(&mut WallpaperSettings)) -> AppResult<()> {
        let state = self.app.state::<AppState>();
        let mut next: Settings = state.settings.lock().unwrap_or_else(|e| e.into_inner()).clone();
        change(&mut next.wallpaper);
        crate::commands::settings::persist_and_broadcast(&state, next).map(|_| ())
    }

    fn is_saved(&self, path: &Path) -> bool {
        path.parent().is_some_and(|p| p == self.saved_dir)
    }

    pub fn snapshot(&self) -> WallpaperState {
        let settings = self.settings();
        let saved: Vec<Wallpaper> = pictures_in(&self.saved_dir, MAX_SAVED)
            .iter()
            .map(|p| describe(p, true, &self.thumbs_dir))
            .collect();
        let folder: Vec<Wallpaper> = settings
            .folder
            .as_deref()
            .map(|f| pictures_in(Path::new(f), MAX_FOLDER))
            .unwrap_or_default()
            .iter()
            .map(|p| describe(p, false, &self.thumbs_dir))
            .collect();
        let recent: Vec<Wallpaper> = settings
            .recent
            .iter()
            .map(PathBuf::from)
            .filter(|p| p.is_file())
            .map(|p| describe(&p, self.is_saved(&p), &self.thumbs_dir))
            .collect();
        let screens = desktop::screens().unwrap_or_else(|error| {
            tracing::warn!(kind = error.kind(), "could not list screens for wallpapers");
            Vec::new()
        });
        let target = resolve_target(&settings.target, &screens).to_owned();
        let missing: Vec<PathBuf> = saved
            .iter()
            .chain(&folder)
            .chain(&recent)
            .filter(|w| w.thumb.is_none())
            .map(|w| PathBuf::from(&w.path))
            .collect();
        self.make_thumbs(missing);
        WallpaperState {
            saved,
            folder,
            recent,
            screens,
            folder_path: settings.folder,
            target,
        }
    }

    fn publish(&self) {
        let state = self.snapshot();
        self.app
            .state::<AppState>()
            .bus
            .emit("wallpaper", AppEvent::WallpaperChanged(state));
    }

    /// Make missing previews in the background, then tell the islands.
    fn make_thumbs(&self, paths: Vec<PathBuf>) {
        let paths: Vec<PathBuf> = {
            let unreadable = self.unreadable.lock().unwrap_or_else(|e| e.into_inner());
            paths.into_iter().filter(|p| !unreadable.contains(p)).collect()
        };
        if paths.is_empty() || self.making_thumbs.swap(true, Ordering::SeqCst) {
            return;
        }
        let this = self.clone();
        tauri::async_runtime::spawn_blocking(move || {
            let mut made = 0;
            for path in &paths {
                let dest = this.thumbs_dir.join(thumb_key(path));
                if dest.exists() {
                    continue;
                }
                match make_thumb(path, &dest, THUMB_HEIGHT) {
                    Ok(()) => made += 1,
                    Err(error) => {
                        tracing::info!(kind = error.kind(), "could not preview a wallpaper");
                        this.unreadable
                            .lock()
                            .unwrap_or_else(|e| e.into_inner())
                            .insert(path.clone());
                    }
                }
            }
            this.making_thumbs.store(false, Ordering::SeqCst);
            if made > 0 {
                this.publish();
            }
        });
    }

    /// Copy pictures in as saved photos; returns how many were new.
    pub fn add(&self, paths: &[String]) -> AppResult<usize> {
        let sources = paths
            .iter()
            .map(|p| check_picture(p))
            .collect::<AppResult<Vec<_>>>()?;
        std::fs::create_dir_all(&self.saved_dir)?;
        let mut saved = pictures_in(&self.saved_dir, usize::MAX);
        let mut added = 0;
        for source in sources {
            if std::fs::metadata(&source).map(|m| m.len()).unwrap_or(0) > MAX_BYTES {
                return Err(AppError::InvalidInput(
                    "That picture is too big (over 60 MB)".into(),
                ));
            }
            if already_saved(&saved, &source) {
                continue;
            }
            if saved.len() >= MAX_SAVED {
                return Err(AppError::InvalidInput(format!(
                    "Overline keeps {MAX_SAVED} wallpapers — remove some first"
                )));
            }
            let dest = self.saved_dir.join(saved_name(&source));
            std::fs::copy(&source, &dest)?;
            saved.push(dest);
            added += 1;
        }
        self.publish();
        Ok(added)
    }

    /// Delete one of Overline's saved copies (never a picture from your folder).
    pub fn remove(&self, path: &str) -> AppResult<()> {
        let p = PathBuf::from(path);
        if !self.is_saved(&p) {
            return Err(AppError::InvalidInput(
                "Only saved wallpapers can be removed".into(),
            ));
        }
        let thumb = self.thumbs_dir.join(thumb_key(&p));
        std::fs::remove_file(&p)?;
        let _ = std::fs::remove_file(thumb);
        self.publish();
        Ok(())
    }

    pub fn set_folder(&self, folder: Option<String>) -> AppResult<()> {
        if let Some(f) = &folder {
            let p = Path::new(f);
            if f.chars().count() > super::MAX_PATH_CHARS || !p.is_absolute() || !p.is_dir() {
                return Err(AppError::InvalidInput("That folder doesn't exist".into()));
            }
        }
        self.update(|w| w.folder = folder)?;
        self.publish();
        Ok(())
    }

    pub fn set_target(&self, target: String) -> AppResult<()> {
        if target != ALL && target.chars().count() > super::MAX_PATH_CHARS {
            return Err(AppError::InvalidInput("invalid screen".into()));
        }
        self.update(|w| w.target = target)?;
        self.publish();
        Ok(())
    }

    pub fn clear_recent(&self) -> AppResult<()> {
        self.update(|w| w.recent.clear())?;
        self.publish();
        Ok(())
    }

    /// Two colours from the main screen's wallpaper (for the notch's accent), if it has one.
    pub fn accent(&self) -> Option<crate::music::Palette> {
        let screens = desktop::screens().ok()?;
        let screen = screens.iter().find(|s| s.primary).or(screens.first())?;
        let path = PathBuf::from(screen.current.as_ref()?);
        let key = thumb_key(&path);
        let mut cache = self.accent_cache.lock().unwrap_or_else(|e| e.into_inner());
        if let Some((k, palette)) = cache.as_ref() {
            if *k == key {
                return palette.clone();
            }
        }
        let palette = (|| {
            let picture = image::ImageReader::open(&path)
                .ok()?
                .with_guessed_format()
                .ok()?
                .decode()
                .ok()?;
            let small = picture.thumbnail(160, 160).to_rgb8();
            let mut png = Vec::new();
            small
                .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
                .ok()?;
            crate::music::palette(&png)
        })();
        *cache = Some((key, palette.clone()));
        palette
    }

    /// Set a picture as the wallpaper on `target` (`all` or a screen id).
    pub fn apply(&self, path: &str, target: &str) -> AppResult<()> {
        let picture = check_picture(path)?;
        let screens = desktop::screens()?;
        let target = resolve_target(target, &screens);
        desktop::apply(&picture.display().to_string(), target)?;
        let path = picture.display().to_string();
        self.update(|w| w.recent = push_recent(&w.recent, &path))?;
        self.publish();
        // Windows reports the new wallpaper a moment later; refresh the check mark then.
        let this = self.clone();
        tauri::async_runtime::spawn_blocking(move || {
            std::thread::sleep(std::time::Duration::from_millis(1200));
            this.publish();
        });
        Ok(())
    }
}
