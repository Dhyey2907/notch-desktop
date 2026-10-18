//! Screenshots to the shelf: Windows saves every Win+Shift+S / Print Screen capture to
//! Pictures\Screenshots; each new one lands on the shelf, and the notch offers to copy its
//! text, mark it up, or send it to your phone.

use std::{
    collections::HashSet,
    path::{Path, PathBuf},
    sync::mpsc::channel,
    time::Duration,
};

use notify::{EventKind, RecursiveMode, Watcher};

/// Is this a finished screenshot file worth taking (not a temp file Windows writes first)?
pub fn is_screenshot(path: &Path) -> bool {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    matches!(ext.as_str(), "png" | "jpg" | "jpeg") && !name.starts_with('~') && !name.starts_with('.')
}

/// Where Windows saves screenshots (Pictures\Screenshots, wherever Pictures is, OneDrive too).
#[cfg(windows)]
pub fn folder() -> Option<PathBuf> {
    use windows::Win32::{
        System::Com::CoTaskMemFree,
        UI::Shell::{FOLDERID_Screenshots, SHGetKnownFolderPath, KF_FLAG_DEFAULT},
    };
    // SAFETY: Windows allocates the path; it's copied and freed here.
    unsafe {
        let p = SHGetKnownFolderPath(&FOLDERID_Screenshots, KF_FLAG_DEFAULT, None).ok()?;
        let path = p.to_string().ok();
        CoTaskMemFree(Some(p.0 as *const _));
        path.map(PathBuf::from)
    }
    .or_else(|| {
        std::env::var_os("USERPROFILE").map(|h| PathBuf::from(h).join("Pictures").join("Screenshots"))
    })
}

#[cfg(not(windows))]
pub fn folder() -> Option<PathBuf> {
    None
}

/// Watch the screenshots folder and call `found` once per new screenshot.
pub fn watch(found: impl Fn(PathBuf) + Send + 'static) {
    let Some(dir) = folder() else { return };
    if std::fs::create_dir_all(&dir).is_err() {
        return;
    }
    std::thread::Builder::new()
        .name("screenshots".into())
        .spawn(move || {
            let (tx, rx) = channel();
            let mut watcher = match notify::recommended_watcher(move |e: notify::Result<notify::Event>| {
                if let Ok(e) = e {
                    if matches!(e.kind, EventKind::Create(_) | EventKind::Modify(_)) {
                        for p in e.paths {
                            let _ = tx.send(p);
                        }
                    }
                }
            }) {
                Ok(w) => w,
                Err(error) => {
                    tracing::warn!(%error, "could not watch the Screenshots folder");
                    return;
                }
            };
            if let Err(error) = watcher.watch(&dir, RecursiveMode::NonRecursive) {
                tracing::warn!(%error, "could not watch the Screenshots folder");
                return;
            }
            let mut seen: HashSet<PathBuf> = HashSet::new();
            while let Ok(path) = rx.recv() {
                if !is_screenshot(&path) || seen.contains(&path) {
                    continue;
                }
                // Let Windows finish writing it.
                std::thread::sleep(Duration::from_millis(600));
                if std::fs::metadata(&path).map(|m| m.len() > 0).unwrap_or(false) {
                    seen.insert(path.clone());
                    found(path);
                }
            }
        })
        .ok();
}

/// Text in an image file (Windows' own text recognition).
#[cfg(windows)]
pub fn read_text(path: &Path) -> crate::error::AppResult<String> {
    use crate::error::AppError;
    let img = image::open(path)
        .map_err(|e| AppError::InvalidInput(format!("couldn't open the picture: {e}")))?
        .to_rgba8();
    let (w, h) = img.dimensions();
    let mut bgra = img.into_raw();
    for px in bgra.chunks_exact_mut(4) {
        px.swap(0, 2);
    }
    let (w, h, bgra) = if h < 60 {
        crate::grab::upscale2(w, h, &bgra)
    } else {
        (w, h, bgra)
    };
    crate::grab::ocr::read(w, h, &bgra)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn takes_finished_pictures_only() {
        assert!(is_screenshot(Path::new(
            "C:\\p\\Screenshot 2026-10-18 101500.png"
        )));
        assert!(is_screenshot(Path::new("C:\\p\\shot.JPG")));
        assert!(!is_screenshot(Path::new("C:\\p\\~tmp1234.png")));
        assert!(!is_screenshot(Path::new("C:\\p\\desktop.ini")));
    }
}
