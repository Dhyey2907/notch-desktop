//! Files: downloads in progress, recently opened files, shelf tools (convert, shrink, merge)
//! and recording what the PC plays.

pub mod convert;
#[cfg(windows)]
mod downloads;
#[cfg(windows)]
pub(crate) mod recent;
#[cfg(windows)]
pub(crate) mod recorder;
pub mod screenrec;

/// The screen recording now, for the bubble's timer.
#[derive(Debug, Clone, Default, serde::Serialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ScreenRecState {
    /// When it started (Unix ms); none when not recording.
    #[ts(optional, type = "number")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub started_at: Option<i64>,
    pub gif: bool,
}
pub mod service;

use std::path::{Path, PathBuf};

use serde::Serialize;
use ts_rs::TS;

/// A download that's still being written.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct DownloadProgress {
    /// The name it will have when done (or "Download" while the browser hasn't named it).
    pub name: String,
    #[ts(type = "number")]
    pub bytes: u64,
    /// Bytes per second.
    #[ts(type = "number")]
    pub speed: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct DownloadDone {
    pub name: String,
    pub path: String,
    #[ts(type = "number")]
    pub size: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct RecentFile {
    pub path: String,
    pub name: String,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ext: Option<String>,
    #[ts(type = "number")]
    pub size: u64,
    /// When it was last opened (epoch ms).
    #[ts(type = "number")]
    pub opened_at: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct RecorderState {
    pub recording: bool,
    /// Epoch ms when the current recording started.
    #[ts(optional, type = "number")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub started_at: Option<i64>,
}

/// Temporary names browsers give downloads in progress.
const PARTIAL: &[&str] = &[".crdownload", ".part", ".partial", ".download", ".opdownload"];

/// The finished name for a download in progress, or `None` if it isn't one.
/// `Unconfirmed 12345.crdownload` (Chrome before it knows the name) gives `Some("")`.
pub fn final_name(file_name: &str) -> Option<String> {
    let lower = file_name.to_lowercase();
    let ext = PARTIAL.iter().find(|ext| lower.ends_with(*ext))?;
    let base = &file_name[..file_name.len() - ext.len()];
    // Firefox: "name.zip.part"; Firefox also uses "abc123.zip.part" with a random prefix,
    // and Chrome "Unconfirmed 123.crdownload" — neither is the real name yet.
    if base.starts_with("Unconfirmed ") || base.is_empty() {
        return Some(String::new());
    }
    Some(base.to_owned())
}

/// A finished download (a file in the Downloads folder).
#[derive(Debug, Clone, serde::Serialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct RecentDownload {
    pub name: String,
    pub path: String,
    #[ts(type = "number")]
    pub bytes: u64,
    /// When it finished (last written), Unix ms.
    #[ts(type = "number")]
    pub at: i64,
}

/// Files in `dir` finished in the last `hours` (no partial downloads), newest first.
pub fn recent_downloads(
    dir: &std::path::Path,
    now: std::time::SystemTime,
    hours: u64,
    max: usize,
) -> Vec<RecentDownload> {
    let window = std::time::Duration::from_secs(hours * 3600);
    let mut out: Vec<RecentDownload> = std::fs::read_dir(dir)
        .map(|entries| {
            entries
                .flatten()
                .filter_map(|e| {
                    let name = e.file_name().to_string_lossy().into_owned();
                    if final_name(&name).is_some()
                        || name.eq_ignore_ascii_case("desktop.ini")
                        || name.starts_with('.')
                    {
                        return None;
                    }
                    let meta = e.metadata().ok()?;
                    if !meta.is_file() {
                        return None;
                    }
                    let modified = meta.modified().ok()?;
                    if now.duration_since(modified).is_ok_and(|age| age > window) {
                        return None;
                    }
                    let at = modified
                        .duration_since(std::time::UNIX_EPOCH)
                        .map_or(0, |d| d.as_millis() as i64);
                    Some(RecentDownload {
                        name,
                        path: e.path().display().to_string(),
                        bytes: meta.len(),
                        at,
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    out.sort_by_key(|d| std::cmp::Reverse(d.at));
    out.truncate(max);
    out
}

/// `dir/name` if free, else `dir/name (2)` and so on.
pub fn unique_path(path: PathBuf) -> PathBuf {
    let dir = path.parent().map(Path::to_path_buf).unwrap_or_default();
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    crate::bridge::unique_path(&dir, &name)
}

/// Bytes per second between two sizes `millis` apart.
pub fn speed(before: u64, after: u64, millis: u64) -> u64 {
    if millis == 0 || after <= before {
        0
    } else {
        (after - before) * 1000 / millis
    }
}

/// "Recording 2026-10-01 20.15.mp3".
pub fn recording_name(now: chrono::DateTime<chrono::Local>, ext: &str) -> String {
    format!("Recording {}.{ext}", now.format("%Y-%m-%d %H.%M"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lists_finished_downloads_newest_first() {
        let dir = std::env::temp_dir().join(format!("overline-dl-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("report.pdf"), b"12345").unwrap();
        std::fs::write(dir.join("movie.mp4.crdownload"), b"x").unwrap();
        std::fs::write(dir.join("desktop.ini"), b"x").unwrap();
        let list = recent_downloads(&dir, std::time::SystemTime::now(), 24, 10);
        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].name, "report.pdf");
        assert_eq!(list[0].bytes, 5);
    }

    #[test]
    fn recognizes_downloads_in_progress() {
        assert_eq!(final_name("setup.exe.crdownload"), Some("setup.exe".into()));
        assert_eq!(final_name("Movie.MKV.PART"), Some("Movie.MKV".into()));
        assert_eq!(final_name("Unconfirmed 812.crdownload"), Some(String::new()));
        assert_eq!(final_name(".part"), Some(String::new()));
        assert_eq!(final_name("report.pdf"), None);
        assert_eq!(final_name("party.partition"), None);
    }

    #[test]
    fn measures_speed() {
        assert_eq!(speed(0, 5_000_000, 1000), 5_000_000);
        assert_eq!(speed(100, 50, 1000), 0);
        assert_eq!(speed(0, 10, 0), 0);
    }

    #[test]
    fn names_recordings() {
        use chrono::TimeZone;
        let at = chrono::Local.with_ymd_and_hms(2026, 10, 1, 20, 15, 9).unwrap();
        assert_eq!(recording_name(at, "mp3"), "Recording 2026-10-01 20.15.mp3");
    }
}
