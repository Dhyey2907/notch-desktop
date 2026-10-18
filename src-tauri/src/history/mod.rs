//! Notch history: "what did I miss?" — the cards the notch showed in the last two days, kept on
//! this PC. For modules that show private things (messages, mail, the clipboard, passwords,
//! AI answers) only the card's title is kept, never its text.

use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::error::AppResult;

/// Cards older than this are dropped.
pub const KEEP_HOURS: i64 = 48;
/// Modules whose card text is never stored.
const TITLE_ONLY: &[&str] = &["phone", "mail", "clipboard", "vault", "ask", "rewrite", "capture"];

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct HistoryEntry {
    /// Unix ms.
    #[ts(type = "number")]
    pub at: i64,
    pub module: String,
    pub title: String,
    #[ts(optional)]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub body: Option<String>,
    #[ts(optional)]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tone: Option<String>,
}

/// What actually gets stored: trimmed, and no text for private modules.
pub fn sanitize(mut e: HistoryEntry) -> Option<HistoryEntry> {
    e.module = e.module.chars().take(40).collect();
    e.title = e.title.trim().chars().take(160).collect();
    if e.title.is_empty() {
        return None;
    }
    e.body = if TITLE_ONLY.contains(&e.module.as_str()) {
        None
    } else {
        e.body
            .map(|b| b.trim().chars().take(400).collect::<String>())
            .filter(|b| !b.is_empty())
    };
    e.tone = e.tone.map(|t| t.chars().take(16).collect());
    Some(e)
}

pub fn add(conn: &Connection, entry: HistoryEntry, now_ms: i64) -> AppResult<()> {
    let Some(e) = sanitize(entry) else { return Ok(()) };
    conn.execute(
        "INSERT INTO activity_log (at, module, title, body, tone) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![e.at, e.module, e.title, e.body, e.tone],
    )?;
    conn.execute(
        "DELETE FROM activity_log WHERE at < ?1",
        params![now_ms - KEEP_HOURS * 3_600_000],
    )?;
    Ok(())
}

/// Newest first, from the last `hours`.
pub fn list(conn: &Connection, now_ms: i64, hours: i64) -> AppResult<Vec<HistoryEntry>> {
    let mut stmt = conn.prepare(
        "SELECT at, module, title, body, tone FROM activity_log WHERE at >= ?1 ORDER BY at DESC LIMIT 300",
    )?;
    let rows = stmt.query_map(params![now_ms - hours.clamp(1, KEEP_HOURS) * 3_600_000], |r| {
        Ok(HistoryEntry {
            at: r.get(0)?,
            module: r.get(1)?,
            title: r.get(2)?,
            body: r.get(3)?,
            tone: r.get(4)?,
        })
    })?;
    Ok(rows.filter_map(Result::ok).collect())
}

pub fn clear(conn: &Connection) -> AppResult<()> {
    conn.execute("DELETE FROM activity_log", [])?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(include_str!("../db/migrations/017_activity_log.sql"))
            .unwrap();
        conn
    }

    fn entry(module: &str, at: i64) -> HistoryEntry {
        HistoryEntry {
            at,
            module: module.into(),
            title: "Title".into(),
            body: Some("secret body".into()),
            tone: None,
        }
    }

    #[test]
    fn keeps_no_text_for_private_modules() {
        let conn = db();
        let now = 10 * 3_600_000;
        add(&conn, entry("phone", now), now).unwrap();
        add(&conn, entry("calendar", now - 1), now).unwrap();
        let l = list(&conn, now, 48).unwrap();
        assert_eq!(l.len(), 2);
        assert_eq!(l[0].module, "phone");
        assert!(l[0].body.is_none());
        assert_eq!(l[1].body.as_deref(), Some("secret body"));
    }

    #[test]
    fn drops_old_cards_and_empty_titles() {
        let conn = db();
        let now = 100 * 3_600_000;
        add(&conn, entry("day", now - 49 * 3_600_000), now - 49 * 3_600_000).unwrap();
        add(&conn, entry("day", now), now).unwrap();
        let mut blank = entry("day", now);
        blank.title = "  ".into();
        add(&conn, blank, now).unwrap();
        assert_eq!(list(&conn, now, 48).unwrap().len(), 1);
        clear(&conn).unwrap();
        assert!(list(&conn, now, 48).unwrap().is_empty());
    }
}
