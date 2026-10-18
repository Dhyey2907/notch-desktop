//! Wallpapers: photos you save in Overline (it keeps its own copies) and the pictures in a
//! folder you choose, set as the desktop wallpaper on one screen or all of them in a click.
//! File names are never logged.

pub mod service;
#[cfg(windows)]
pub(crate) mod win;

use std::{
    hash::{Hash, Hasher},
    path::{Path, PathBuf},
    time::SystemTime,
};

use serde::Serialize;
use ts_rs::TS;

use crate::error::{AppError, AppResult};

/// Saved photos Overline keeps.
pub const MAX_SAVED: usize = 60;
/// Pictures shown from the chosen folder (newest first).
pub const MAX_FOLDER: usize = 200;
/// Wallpapers remembered under "Recent".
pub const MAX_RECENT: usize = 12;
/// Bigger files aren't copied in (a wallpaper is rarely more than a few MB).
pub const MAX_BYTES: u64 = 60 * 1024 * 1024;
pub const MAX_PATH_CHARS: usize = 1024;
/// "Every screen" as a target.
pub const ALL: &str = "all";
const EXTENSIONS: [&str; 5] = ["jpg", "jpeg", "png", "webp", "bmp"];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Wallpaper {
    /// Full path of the picture (also its id).
    pub path: String,
    pub name: String,
    /// Small JPEG preview, once made.
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thumb: Option<String>,
    /// One of the photos Overline keeps (can be removed); otherwise from the folder.
    pub saved: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct WallpaperScreen {
    /// Windows' id for the monitor (a device path).
    pub id: String,
    /// "Screen 1", numbered like Windows' display settings.
    pub label: String,
    pub primary: bool,
    /// The picture on it now.
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct WallpaperState {
    /// Newest first.
    pub saved: Vec<Wallpaper>,
    /// The chosen folder's pictures, newest first.
    pub folder: Vec<Wallpaper>,
    /// Most recently applied first.
    pub recent: Vec<Wallpaper>,
    pub screens: Vec<WallpaperScreen>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub folder_path: Option<String>,
    /// `all` or a screen id.
    pub target: String,
}

pub fn is_picture(path: &Path) -> bool {
    path.extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .is_some_and(|e| EXTENSIONS.contains(&e.as_str()))
}

/// An absolute path to an existing picture.
pub fn check_picture(path: &str) -> AppResult<PathBuf> {
    let p = Path::new(path);
    if path.chars().count() > MAX_PATH_CHARS || !p.is_absolute() || path.contains('"') {
        return Err(AppError::InvalidInput("That isn't a file path".into()));
    }
    if !is_picture(p) {
        return Err(AppError::InvalidInput(
            "Wallpapers can be JPG, PNG, WebP or BMP pictures".into(),
        ));
    }
    let meta =
        std::fs::metadata(p).map_err(|_| AppError::InvalidInput("That picture doesn't exist".into()))?;
    if !meta.is_file() {
        return Err(AppError::InvalidInput("That picture doesn't exist".into()));
    }
    Ok(p.to_path_buf())
}

fn name_of(path: &Path) -> String {
    path.file_stem()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default()
}

/// When a file arrived: creation time (a copy gets a new one), else last change.
fn arrived(meta: &std::fs::Metadata) -> SystemTime {
    meta.created()
        .or_else(|_| meta.modified())
        .unwrap_or(SystemTime::UNIX_EPOCH)
}

/// Pictures directly in `dir`, newest first, at most `limit`.
pub fn pictures_in(dir: &Path, limit: usize) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut found: Vec<(SystemTime, PathBuf)> = entries
        .flatten()
        .filter_map(|entry| {
            let path = entry.path();
            let meta = entry.metadata().ok()?;
            (meta.is_file() && is_picture(&path)).then(|| (arrived(&meta), path))
        })
        .collect();
    found.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.cmp(&b.1)));
    found.into_iter().take(limit).map(|(_, p)| p).collect()
}

/// Where a copy of `source` goes in the saved folder: its name plus a short random tag, so
/// two photos called "image.jpg" don't collide.
pub fn saved_name(source: &Path) -> String {
    let stem: String = name_of(source)
        .chars()
        .filter(|c| c.is_alphanumeric() || matches!(c, '-' | '_' | ' '))
        .take(40)
        .collect();
    let stem = if stem.trim().is_empty() {
        "wallpaper".to_owned()
    } else {
        stem.trim().to_owned()
    };
    let ext = source
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_else(|| "jpg".into());
    let tag = &uuid::Uuid::new_v4().simple().to_string()[..8];
    format!("{stem}-{tag}.{ext}")
}

/// Same picture already saved: same size and same bytes.
pub fn already_saved(saved: &[PathBuf], source: &Path) -> bool {
    let Ok(len) = std::fs::metadata(source).map(|m| m.len()) else {
        return false;
    };
    saved.iter().any(|p| {
        std::fs::metadata(p).is_ok_and(|m| m.len() == len)
            && std::fs::read(p).ok() == std::fs::read(source).ok()
    })
}

/// The thumbnail's file name: changes when the picture does.
pub fn thumb_key(path: &Path) -> String {
    let mut h = std::collections::hash_map::DefaultHasher::new();
    path.hash(&mut h);
    if let Ok(meta) = std::fs::metadata(path) {
        meta.len().hash(&mut h);
        meta.modified().ok().hash(&mut h);
    }
    format!("{:016x}.jpg", h.finish())
}

/// Put `path` first in the recent list, without duplicates, at most `MAX_RECENT`.
pub fn push_recent(recent: &[String], path: &str) -> Vec<String> {
    std::iter::once(path.to_owned())
        .chain(recent.iter().filter(|p| !p.eq_ignore_ascii_case(path)).cloned())
        .take(MAX_RECENT)
        .collect()
}

/// A target is `all` or one of the connected screens' ids; anything else falls back to `all`.
pub fn resolve_target<'a>(target: &'a str, screens: &[WallpaperScreen]) -> &'a str {
    if screens.iter().any(|s| s.id == target) {
        target
    } else {
        ALL
    }
}

pub fn describe(path: &Path, saved: bool, thumbs: &Path) -> Wallpaper {
    let thumb = thumbs.join(thumb_key(path));
    Wallpaper {
        path: path.display().to_string(),
        name: name_of(path),
        thumb: thumb.exists().then(|| thumb.display().to_string()),
        saved,
    }
}

/// Make a JPEG preview about `height` pixels tall.
pub fn make_thumb(source: &Path, dest: &Path, height: u32) -> AppResult<()> {
    let picture = image::ImageReader::open(source)
        .map_err(|e| AppError::Platform(e.to_string()))?
        .with_guessed_format()
        .map_err(|e| AppError::Platform(e.to_string()))?
        .decode()
        .map_err(|e| AppError::Platform(e.to_string()))?;
    let width = (picture.width() as u64 * height as u64 / picture.height().max(1) as u64).max(1) as u32;
    let small = picture.thumbnail(width, height).to_rgb8();
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut out = std::io::BufWriter::new(std::fs::File::create(dest)?);
    image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, 82)
        .encode_image(&small)
        .map_err(|e| AppError::Platform(e.to_string()))?;
    Ok(())
}

/// Screens, and setting the wallpaper (Windows only).
pub mod desktop {
    use super::WallpaperScreen;
    use crate::error::AppResult;

    pub fn screens() -> AppResult<Vec<WallpaperScreen>> {
        #[cfg(windows)]
        return super::win::screens();
        #[cfg(not(windows))]
        Ok(Vec::new())
    }

    /// Set `path` on the screen with this id, or on every screen for `all`.
    pub fn apply(path: &str, target: &str) -> AppResult<()> {
        #[cfg(windows)]
        return super::win::apply(path, target);
        #[cfg(not(windows))]
        {
            let _ = (path, target);
            Err(crate::error::AppError::Platform("not supported".into()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("overline-wall-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn screen(id: &str) -> WallpaperScreen {
        WallpaperScreen {
            id: id.into(),
            label: "Screen 1".into(),
            primary: true,
            current: None,
        }
    }

    #[test]
    fn only_pictures_count() {
        assert!(is_picture(Path::new("C:/a/b.JPG")));
        assert!(is_picture(Path::new("C:/a/b.webp")));
        assert!(!is_picture(Path::new("C:/a/b.gif")));
        assert!(!is_picture(Path::new("C:/a/b")));
    }

    #[test]
    fn folder_scan_keeps_pictures_newest_first() {
        let dir = temp_dir("scan");
        for name in ["a.jpg", "notes.txt", "b.PNG", "c.webp"] {
            std::fs::write(dir.join(name), name).unwrap();
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        std::fs::create_dir(dir.join("sub.jpg")).unwrap();
        let found: Vec<String> = pictures_in(&dir, 10)
            .iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
            .collect();
        assert_eq!(found, ["c.webp", "b.PNG", "a.jpg"]);
        assert_eq!(pictures_in(&dir, 2).len(), 2);
        assert!(pictures_in(&dir.join("missing"), 10).is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn checks_paths() {
        let dir = temp_dir("check");
        let pic = dir.join("x.jpg");
        std::fs::write(&pic, "x").unwrap();
        assert!(check_picture(&pic.display().to_string()).is_ok());
        assert!(check_picture("relative.jpg").is_err());
        assert!(check_picture(&dir.join("x.txt").display().to_string()).is_err());
        assert!(check_picture(&dir.join("gone.jpg").display().to_string()).is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn saved_names_are_safe_and_unique() {
        let a = saved_name(Path::new("C:/x/My: Photo?.JPEG"));
        let b = saved_name(Path::new("C:/x/My: Photo?.JPEG"));
        assert!(a.starts_with("My Photo-") && a.ends_with(".jpeg"), "{a}");
        assert_ne!(a, b);
        assert!(saved_name(Path::new("C:/x/???.png")).starts_with("wallpaper-"));
    }

    #[test]
    fn spots_duplicates() {
        let dir = temp_dir("dupe");
        let (a, b, c) = (dir.join("a.jpg"), dir.join("b.jpg"), dir.join("c.jpg"));
        std::fs::write(&a, "same").unwrap();
        std::fs::write(&b, "same").unwrap();
        std::fs::write(&c, "diff").unwrap();
        assert!(already_saved(std::slice::from_ref(&a), &b));
        assert!(!already_saved(&[a], &c));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn recent_list_moves_to_front_and_caps() {
        let mut recent: Vec<String> = Vec::new();
        for i in 0..20 {
            recent = push_recent(&recent, &format!("C:/w/{i}.jpg"));
        }
        assert_eq!(recent.len(), MAX_RECENT);
        assert_eq!(recent[0], "C:/w/19.jpg");
        let again = push_recent(&recent, "c:/W/15.JPG");
        assert_eq!(again[0], "c:/W/15.JPG");
        assert_eq!(again.len(), MAX_RECENT);
        assert_eq!(
            again
                .iter()
                .filter(|p| p.eq_ignore_ascii_case("c:/w/15.jpg"))
                .count(),
            1
        );
    }

    #[test]
    fn unknown_targets_mean_all() {
        let screens = [screen("mon-1"), screen("mon-2")];
        assert_eq!(resolve_target("mon-2", &screens), "mon-2");
        assert_eq!(resolve_target("unplugged", &screens), ALL);
        assert_eq!(resolve_target(ALL, &screens), ALL);
    }

    #[test]
    fn thumbnails_are_small_jpegs() {
        let dir = temp_dir("thumb");
        let src = dir.join("big.png");
        image::RgbImage::from_pixel(1200, 800, image::Rgb([200, 30, 30]))
            .save(&src)
            .unwrap();
        let dest = dir.join("t").join(thumb_key(&src));
        make_thumb(&src, &dest, 120).unwrap();
        let small = image::open(&dest).unwrap();
        assert_eq!(small.height(), 120);
        assert_eq!(small.width(), 180);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
