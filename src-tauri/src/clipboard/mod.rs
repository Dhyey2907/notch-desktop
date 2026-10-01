//! Clipboard history: everything you copy (text, pictures, files) in a list you can copy from
//! again. History lives in memory only; pinned clips are kept in SQLite because you chose to
//! keep them. Clip contents are never logged.
//!
//! Copies that apps mark as private (password managers set `ExcludeClipboardContentFromMonitor
//! Processing` or `CanIncludeInClipboardHistory = 0`, the same signals Windows' own clipboard
//! history honors), copies from blocked apps, and copies while paused are skipped.

pub mod image;
pub mod service;
#[cfg(windows)]
pub(crate) mod win;

use std::{
    collections::VecDeque,
    hash::{DefaultHasher, Hash, Hasher},
    path::PathBuf,
};

use chrono::{DateTime, TimeZone, Utc};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::error::{AppError, AppResult};

/// Clips kept in history (pinned ones don't count).
pub const MAX_RECENT: usize = 50;
pub const MAX_PINNED: usize = 100;
/// Longer text isn't kept (a copied log file, say); 1 MB covers any normal copy.
pub const MAX_TEXT_BYTES: usize = 1024 * 1024;
/// Files per clip.
pub const MAX_FILES: usize = 100;
/// Characters of text sent to the notch for the list and search.
const PREVIEW_CHARS: usize = 1000;

/// Password managers are skipped even if they forget to mark their copies private.
const ALWAYS_BLOCKED: &[&str] = &[
    "1password.exe",
    "bitwarden.exe",
    "keepass.exe",
    "keepassxc.exe",
    "lastpass.exe",
    "dashlane.exe",
    "enpass.exe",
    "proton pass.exe",
    "nordpass.exe",
    "roboform.exe",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum ClipKind {
    Text,
    Image,
    Files,
}

/// Recognised text, for quick actions (open, color swatch, format).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum TextKind {
    Url,
    Color,
    Json,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Clip {
    pub id: String,
    pub kind: ClipKind,
    /// Start of the text, or the file names; empty for pictures.
    pub preview: String,
    /// Characters of text (the preview may be shorter).
    pub length: u32,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text_kind: Option<TextKind>,
    /// Full paths, for files.
    pub files: Vec<String>,
    /// PNG on disk, for pictures (shown through the asset protocol).
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<String>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub width: Option<u32>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub height: Option<u32>,
    /// App it was copied from, e.g. "Chrome".
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub app: Option<String>,
    pub at: DateTime<Utc>,
    pub pinned: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ClipboardSnapshot {
    /// Most recently pinned first.
    pub pinned: Vec<Clip>,
    /// Most recently copied first.
    pub recent: Vec<Clip>,
}

/// What a clip holds, in full.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Content {
    Text(String),
    Image(PathBuf),
    Files(Vec<String>),
}

#[derive(Debug, Clone)]
pub struct Entry {
    pub clip: Clip,
    pub content: Content,
    /// Identifies identical copies (text, picture bytes, or file list).
    pub hash: u64,
}

pub fn hash_of(kind: ClipKind, bytes: &[u8]) -> u64 {
    let mut hasher = DefaultHasher::new();
    kind.hash(&mut hasher);
    bytes.hash(&mut hasher);
    hasher.finish()
}

impl Hash for ClipKind {
    fn hash<H: Hasher>(&self, state: &mut H) {
        (*self as u8).hash(state);
    }
}

pub fn text_entry(text: String, app: Option<String>, at: DateTime<Utc>) -> Option<Entry> {
    if text.trim().is_empty() || text.len() > MAX_TEXT_BYTES {
        return None;
    }
    let hash = hash_of(ClipKind::Text, text.as_bytes());
    Some(Entry {
        clip: Clip {
            id: uuid::Uuid::new_v4().to_string(),
            kind: ClipKind::Text,
            preview: text.chars().take(PREVIEW_CHARS).collect(),
            length: text.chars().count().min(u32::MAX as usize) as u32,
            text_kind: detect_text(&text),
            files: Vec::new(),
            image: None,
            width: None,
            height: None,
            app,
            at,
            pinned: false,
        },
        content: Content::Text(text),
        hash,
    })
}

pub fn image_entry(
    path: PathBuf,
    hash: u64,
    (width, height): (u32, u32),
    app: Option<String>,
    at: DateTime<Utc>,
) -> Entry {
    Entry {
        clip: Clip {
            id: uuid::Uuid::new_v4().to_string(),
            kind: ClipKind::Image,
            preview: String::new(),
            length: 0,
            text_kind: None,
            files: Vec::new(),
            image: Some(path.display().to_string()),
            width: Some(width),
            height: Some(height),
            app,
            at,
            pinned: false,
        },
        content: Content::Image(path),
        hash,
    }
}

pub fn files_entry(mut files: Vec<String>, app: Option<String>, at: DateTime<Utc>) -> Option<Entry> {
    files.truncate(MAX_FILES);
    if files.is_empty() {
        return None;
    }
    let hash = hash_of(ClipKind::Files, files.join("\n").as_bytes());
    let names: Vec<&str> = files
        .iter()
        .map(|f| f.rsplit(['\\', '/']).next().unwrap_or(f))
        .collect();
    Some(Entry {
        clip: Clip {
            id: uuid::Uuid::new_v4().to_string(),
            kind: ClipKind::Files,
            preview: names.join(", ").chars().take(PREVIEW_CHARS).collect(),
            length: 0,
            text_kind: None,
            files: files.clone(),
            image: None,
            width: None,
            height: None,
            app,
            at,
            pinned: false,
        },
        content: Content::Files(files),
        hash,
    })
}

/// Links, colors and JSON get quick actions.
pub fn detect_text(text: &str) -> Option<TextKind> {
    let t = text.trim();
    if is_url(t) {
        Some(TextKind::Url)
    } else if is_color(t) {
        Some(TextKind::Color)
    } else if (t.starts_with('{') || t.starts_with('['))
        && serde_json::from_str::<serde_json::Value>(t).is_ok()
    {
        Some(TextKind::Json)
    } else {
        None
    }
}

fn is_url(t: &str) -> bool {
    !t.contains(char::is_whitespace)
        && url::Url::parse(t).is_ok_and(|u| matches!(u.scheme(), "http" | "https") && u.host().is_some())
}

fn is_color(t: &str) -> bool {
    let t = t.to_ascii_lowercase();
    if let Some(hex) = t.strip_prefix('#') {
        return matches!(hex.len(), 3 | 4 | 6 | 8) && hex.chars().all(|c| c.is_ascii_hexdigit());
    }
    ["rgb(", "rgba(", "hsl(", "hsla("].iter().any(|p| {
        t.strip_prefix(p)
            .and_then(|rest| rest.strip_suffix(')'))
            .is_some_and(|args| {
                let parts: Vec<&str> = args.split([',', ' ', '/']).filter(|p| !p.is_empty()).collect();
                (3..=4).contains(&parts.len())
                    && parts
                        .iter()
                        .all(|p| p.trim_end_matches(['%', 'd', 'e', 'g']).parse::<f32>().is_ok())
            })
    })
}

/// JSON, indented for reading.
pub fn pretty_json(text: &str) -> Option<String> {
    let value: serde_json::Value = serde_json::from_str(text.trim()).ok()?;
    serde_json::to_string_pretty(&value).ok()
}

/// "C:\...\chrome.exe" -> "chrome.exe" (lowercase), for block lists.
pub fn exe_key(exe: &str) -> String {
    exe.rsplit(['\\', '/'])
        .next()
        .unwrap_or(exe)
        .trim()
        .to_lowercase()
}

/// "C:\...\chrome.exe" -> "Chrome", for display.
pub fn app_label(exe: &str) -> String {
    let name = exe_key(exe);
    let stem = name.strip_suffix(".exe").unwrap_or(&name);
    let mut chars = stem.chars();
    chars
        .next()
        .map(|c| c.to_uppercase().chain(chars).collect())
        .unwrap_or_default()
}

pub fn is_blocked(exe: Option<&str>, blocked: &[String]) -> bool {
    let Some(exe) = exe else { return false };
    let key = exe_key(exe);
    ALWAYS_BLOCKED.contains(&key.as_str()) || blocked.iter().any(|b| exe_key(b) == key)
}

/// In-memory history plus the pinned clips.
#[derive(Default)]
pub struct History {
    recent: VecDeque<Entry>,
    pinned: Vec<Entry>,
}

impl History {
    pub fn with_pinned(pinned: Vec<Entry>) -> Self {
        Self {
            recent: VecDeque::new(),
            pinned,
        }
    }

    /// Add a copy. The same thing copied again moves to the top instead of repeating.
    /// Returns clips that fell off the end (their pictures can be deleted).
    pub fn add(&mut self, mut entry: Entry) -> Vec<Entry> {
        if self.pinned.iter().any(|p| p.hash == entry.hash) {
            return Vec::new();
        }
        if let Some(i) = self.recent.iter().position(|e| e.hash == entry.hash) {
            let mut existing = self.recent.remove(i).expect("index from position");
            existing.clip.at = entry.clip.at;
            existing.clip.app = entry.clip.app.take().or(existing.clip.app);
            self.recent.push_front(existing);
            return Vec::new();
        }
        self.recent.push_front(entry);
        let mut evicted = Vec::new();
        while self.recent.len() > MAX_RECENT {
            evicted.extend(self.recent.pop_back());
        }
        evicted
    }

    pub fn get(&self, id: &str) -> Option<&Entry> {
        self.pinned
            .iter()
            .chain(self.recent.iter())
            .find(|e| e.clip.id == id)
    }

    /// Copied again from the list: move it to the top (pinned clips stay where they are).
    pub fn bump(&mut self, id: &str, at: DateTime<Utc>) {
        if let Some(i) = self.recent.iter().position(|e| e.clip.id == id) {
            let mut entry = self.recent.remove(i).expect("index from position");
            entry.clip.at = at;
            self.recent.push_front(entry);
        }
    }

    pub fn pin(&mut self, id: &str) -> AppResult<Entry> {
        let i = self
            .recent
            .iter()
            .position(|e| e.clip.id == id)
            .ok_or_else(|| AppError::InvalidInput("That clip is no longer in history".into()))?;
        if self.pinned.len() >= MAX_PINNED {
            return Err(AppError::InvalidInput(
                "That's a lot of pins — unpin some first".into(),
            ));
        }
        let mut entry = self.recent.remove(i).expect("index from position");
        entry.clip.pinned = true;
        self.pinned.insert(0, entry.clone());
        Ok(entry)
    }

    /// Unpinning puts the clip back at the top of history.
    pub fn unpin(&mut self, id: &str) -> AppResult<()> {
        let i = self
            .pinned
            .iter()
            .position(|e| e.clip.id == id)
            .ok_or_else(|| AppError::InvalidInput("That clip isn't pinned".into()))?;
        let mut entry = self.pinned.remove(i);
        entry.clip.pinned = false;
        self.recent.push_front(entry);
        while self.recent.len() > MAX_RECENT {
            self.recent.pop_back();
        }
        Ok(())
    }

    pub fn delete(&mut self, id: &str) -> Option<Entry> {
        if let Some(i) = self.recent.iter().position(|e| e.clip.id == id) {
            return self.recent.remove(i);
        }
        let i = self.pinned.iter().position(|e| e.clip.id == id)?;
        Some(self.pinned.remove(i))
    }

    /// Empty history; pinned clips stay.
    pub fn clear(&mut self) -> Vec<Entry> {
        self.recent.drain(..).collect()
    }

    /// Whether any clip still shows this picture.
    pub fn uses_image(&self, path: &std::path::Path) -> bool {
        self.pinned
            .iter()
            .chain(self.recent.iter())
            .any(|e| matches!(&e.content, Content::Image(p) if p == path))
    }

    pub fn snapshot(&self) -> ClipboardSnapshot {
        ClipboardSnapshot {
            pinned: self.pinned.iter().map(|e| e.clip.clone()).collect(),
            recent: self.recent.iter().map(|e| e.clip.clone()).collect(),
        }
    }
}

// Pinned clips in SQLite.

pub fn save_pin(conn: &Connection, entry: &Entry, now: DateTime<Utc>) -> AppResult<()> {
    let (text, files, image) = match &entry.content {
        Content::Text(t) => (Some(t.as_str()), None, None),
        Content::Files(f) => (None, Some(serde_json::to_string(f).unwrap_or_default()), None),
        Content::Image(p) => (None, None, Some(p.display().to_string())),
    };
    let clip = &entry.clip;
    conn.execute(
        "INSERT OR REPLACE INTO clip_pins
            (id, kind, text, files, image, width, height, app, copied_at, pinned_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        params![
            clip.id,
            kind_name(clip.kind),
            text,
            files,
            image,
            clip.width,
            clip.height,
            clip.app,
            clip.at.timestamp(),
            now.timestamp()
        ],
    )?;
    Ok(())
}

pub fn delete_pin(conn: &Connection, id: &str) -> AppResult<()> {
    conn.execute("DELETE FROM clip_pins WHERE id = ?1", [id])?;
    Ok(())
}

pub fn load_pins(conn: &Connection) -> AppResult<Vec<Entry>> {
    let mut stmt = conn.prepare(
        "SELECT id, kind, text, files, image, width, height, app, copied_at
         FROM clip_pins ORDER BY pinned_at DESC, rowid DESC",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, Option<String>>(2)?,
            row.get::<_, Option<String>>(3)?,
            row.get::<_, Option<String>>(4)?,
            row.get::<_, Option<u32>>(5)?,
            row.get::<_, Option<u32>>(6)?,
            row.get::<_, Option<String>>(7)?,
            row.get::<_, i64>(8)?,
        ))
    })?;
    let mut pins = Vec::new();
    for row in rows {
        let (id, kind, text, files, image, width, height, app, at) = row?;
        let at = Utc.timestamp_opt(at, 0).single().unwrap_or_default();
        let entry = match kind.as_str() {
            "text" => text.and_then(|t| text_entry(t, app, at)),
            "files" => files
                .and_then(|f| serde_json::from_str::<Vec<String>>(&f).ok())
                .and_then(|f| files_entry(f, app, at)),
            "image" => image.map(|p| {
                let bytes = std::fs::read(&p).unwrap_or_default();
                let hash = hash_of(ClipKind::Image, &bytes);
                image_entry(
                    PathBuf::from(p),
                    hash,
                    (width.unwrap_or(0), height.unwrap_or(0)),
                    app,
                    at,
                )
            }),
            _ => None,
        };
        if let Some(mut entry) = entry {
            entry.clip.id = id;
            entry.clip.pinned = true;
            pins.push(entry);
        }
    }
    Ok(pins)
}

fn kind_name(kind: ClipKind) -> &'static str {
    match kind {
        ClipKind::Text => "text",
        ClipKind::Image => "image",
        ClipKind::Files => "files",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text(t: &str) -> Entry {
        text_entry(t.into(), Some("Notepad".into()), Utc::now()).unwrap()
    }

    #[test]
    fn recognises_links_colors_and_json() {
        assert_eq!(detect_text(" https://github.com/x/y "), Some(TextKind::Url));
        assert_eq!(detect_text("http://localhost:1420/"), Some(TextKind::Url));
        assert_eq!(detect_text("see https://x.com"), None);
        assert_eq!(detect_text("#1f2A3b"), Some(TextKind::Color));
        assert_eq!(detect_text("#abc"), Some(TextKind::Color));
        assert_eq!(detect_text("rgb(10, 20, 30)"), Some(TextKind::Color));
        assert_eq!(detect_text("hsl(210deg 40% 50%)"), Some(TextKind::Color));
        assert_eq!(detect_text("#12345"), None);
        assert_eq!(detect_text(r#"{"a": [1, 2]}"#), Some(TextKind::Json));
        assert_eq!(detect_text("{ not json"), None);
        assert_eq!(detect_text("hello"), None);
        assert_eq!(detect_text("javascript:alert(1)"), None);
    }

    #[test]
    fn formats_json() {
        assert_eq!(pretty_json(r#"{"a":1}"#).unwrap(), "{\n  \"a\": 1\n}");
        assert!(pretty_json("nope").is_none());
    }

    #[test]
    fn skips_empty_and_huge_text() {
        assert!(text_entry("   \n".into(), None, Utc::now()).is_none());
        assert!(text_entry("x".repeat(MAX_TEXT_BYTES + 1), None, Utc::now()).is_none());
        let long = text_entry("é".repeat(5000), None, Utc::now()).unwrap();
        assert_eq!(long.clip.preview.chars().count(), PREVIEW_CHARS);
        assert_eq!(long.clip.length, 5000);
    }

    #[test]
    fn copying_the_same_thing_again_moves_it_to_the_top() {
        let mut h = History::default();
        h.add(text("a"));
        h.add(text("b"));
        let first_id = h.snapshot().recent[1].id.clone();
        h.add(text("a"));
        let recent = h.snapshot().recent;
        assert_eq!(recent.len(), 2);
        assert_eq!(recent[0].preview, "a");
        assert_eq!(recent[0].id, first_id, "keeps its id so the list doesn't jump");
    }

    #[test]
    fn keeps_only_the_latest_clips() {
        let mut h = History::default();
        let mut evicted = Vec::new();
        for i in 0..MAX_RECENT + 3 {
            evicted.extend(h.add(text(&format!("clip {i}"))));
        }
        assert_eq!(h.snapshot().recent.len(), MAX_RECENT);
        assert_eq!(evicted.len(), 3);
        assert_eq!(evicted[0].clip.preview, "clip 0");
    }

    #[test]
    fn pinning_moves_a_clip_out_of_history_and_back() {
        let mut h = History::default();
        h.add(text("keep me"));
        let id = h.snapshot().recent[0].id.clone();
        let pinned = h.pin(&id).unwrap();
        assert!(pinned.clip.pinned);
        assert_eq!(h.snapshot().pinned.len(), 1);
        assert!(h.snapshot().recent.is_empty());
        // Copying a pinned clip again doesn't duplicate it into history.
        h.add(text("keep me"));
        assert!(h.snapshot().recent.is_empty());
        h.unpin(&id).unwrap();
        assert_eq!(h.snapshot().recent[0].id, id);
        assert!(!h.snapshot().recent[0].pinned);
    }

    #[test]
    fn clear_keeps_pins() {
        let mut h = History::default();
        h.add(text("a"));
        h.add(text("b"));
        let id = h.snapshot().recent[0].id.clone();
        h.pin(&id).unwrap();
        assert_eq!(h.clear().len(), 1);
        assert_eq!(h.snapshot().pinned.len(), 1);
    }

    #[test]
    fn blocks_password_managers_and_chosen_apps() {
        assert!(is_blocked(Some(r"C:\Program Files\1Password\1Password.exe"), &[]));
        assert!(is_blocked(Some("KeePassXC.exe"), &[]));
        assert!(is_blocked(Some(r"C:\x\Slack.exe"), &["slack.exe".into()]));
        assert!(!is_blocked(Some(r"C:\x\chrome.exe"), &["slack.exe".into()]));
        assert!(!is_blocked(None, &["slack.exe".into()]));
        assert_eq!(app_label(r"C:\Program Files\Google\chrome.exe"), "Chrome");
    }

    #[test]
    fn pins_survive_a_restart() {
        let mut conn = Connection::open_in_memory().unwrap();
        crate::db::migrate_for_tests(&mut conn);
        let mut h = History::default();
        h.add(text("pinned text"));
        h.add(files_entry(vec![r"C:\a\report.pdf".into()], None, Utc::now()).unwrap());
        for clip in h.snapshot().recent {
            let entry = h.pin(&clip.id).unwrap();
            save_pin(&conn, &entry, Utc::now()).unwrap();
        }
        let loaded = load_pins(&conn).unwrap();
        assert_eq!(loaded.len(), 2);
        assert!(loaded.iter().all(|e| e.clip.pinned));
        assert!(loaded
            .iter()
            .any(|e| e.content == Content::Files(vec![r"C:\a\report.pdf".into()])));
        assert!(loaded
            .iter()
            .any(|e| e.content == Content::Text("pinned text".into())));
        delete_pin(&conn, &loaded[0].clip.id).unwrap();
        assert_eq!(load_pins(&conn).unwrap().len(), 1);
    }
}
