//! Gmail glance: the unread count and the latest unread messages (sender, subject, snippet),
//! with Archive and Mark read. Uses the Google sign-in from Calendar, with the Gmail permission
//! the user opted into. Message text is never logged or stored.

pub mod service;

use chrono::{DateTime, TimeZone, Utc};
use serde::Serialize;
use serde_json::Value;
use ts_rs::TS;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct MailMessage {
    pub id: String,
    pub thread_id: String,
    /// "Ada Lovelace" from `"Ada Lovelace" <ada@example.com>`.
    pub from: String,
    pub subject: String,
    pub snippet: String,
    pub at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum MailStatus {
    /// Gmail isn't turned on in Settings.
    Off,
    /// Turned on, but Google hasn't granted Gmail yet (sign in again).
    NeedsSignIn,
    Ready,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct MailState {
    pub status: MailStatus,
    pub unread: u32,
    pub messages: Vec<MailMessage>,
    /// Unread messages hidden until their snooze ends.
    pub snoozed: u32,
}

/// Snoozed message ids: (still hidden, due now). Ids due back come out of the table.
pub fn snoozes(
    conn: &rusqlite::Connection,
    now: DateTime<Utc>,
) -> crate::error::AppResult<(std::collections::HashSet<String>, Vec<String>)> {
    let mut hidden = std::collections::HashSet::new();
    let mut due = Vec::new();
    let mut stmt = conn.prepare("SELECT id, until FROM mail_snoozed")?;
    let rows = stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?;
    for row in rows {
        let (id, until) = row?;
        match DateTime::parse_from_rfc3339(&until) {
            Ok(until) if until > now => {
                hidden.insert(id);
            }
            _ => due.push(id),
        }
    }
    for id in &due {
        conn.execute("DELETE FROM mail_snoozed WHERE id = ?1", [id])?;
    }
    Ok((hidden, due))
}

/// Hide a message until `until` (in the future, within 90 days).
pub fn snooze(
    conn: &rusqlite::Connection,
    id: &str,
    until: DateTime<Utc>,
    now: DateTime<Utc>,
) -> crate::error::AppResult<()> {
    use crate::error::AppError;
    if id.is_empty() || id.len() > 64 || !id.chars().all(|c| c.is_ascii_alphanumeric()) {
        return Err(AppError::InvalidInput("invalid message".into()));
    }
    if until <= now || until - now > chrono::Duration::days(90) {
        return Err(AppError::InvalidInput("Pick a time in the next 90 days".into()));
    }
    conn.execute(
        "INSERT INTO mail_snoozed (id, until) VALUES (?1, ?2)
         ON CONFLICT(id) DO UPDATE SET until = excluded.until",
        rusqlite::params![id, until.to_rfc3339()],
    )?;
    Ok(())
}

/// The sample inbox for demo mode (recording the website's videos).
pub fn demo_messages(now: DateTime<Utc>) -> Vec<MailMessage> {
    let m = |id: &str, from: &str, subject: &str, snippet: &str, minutes: i64| MailMessage {
        id: id.into(),
        thread_id: id.into(),
        from: from.into(),
        subject: subject.into(),
        snippet: snippet.into(),
        at: now - chrono::Duration::minutes(minutes),
    };
    vec![
        m(
            "demo1",
            "Ann Patel",
            "Notes from Tuesday",
            "Here's the summary and the next steps we agreed on…",
            4,
        ),
        m(
            "demo2",
            "Focus Weekly",
            "10 tips for deep work",
            "This week: music, fewer tabs and the two-minute rule…",
            38,
        ),
        m(
            "demo3",
            "Sam Rivera",
            "Lunch on Friday?",
            "There's a new place near the office — 1 pm works for me…",
            95,
        ),
        m(
            "demo4",
            "GitHub",
            "Your weekly digest",
            "3 repositories had new activity this week…",
            180,
        ),
    ]
}

/// The display name from a From header.
pub fn sender_name(from: &str) -> String {
    let from = from.trim();
    if let Some((name, _)) = from.split_once('<') {
        let name = name.trim().trim_matches('"').trim();
        if !name.is_empty() {
            return name.to_owned();
        }
    }
    from.trim_matches(['<', '>']).to_owned()
}

/// Gmail snippets come HTML-escaped.
pub fn unescape(text: &str) -> String {
    text.replace("&#39;", "'")
        .replace("&quot;", "\"")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&nbsp;", " ")
        .replace("&amp;", "&")
}

/// One message from `messages.get?format=metadata`.
pub fn parse_message(json: &Value) -> Option<MailMessage> {
    let header = |name: &str| {
        json["payload"]["headers"]
            .as_array()?
            .iter()
            .find(|h| h["name"].as_str().is_some_and(|n| n.eq_ignore_ascii_case(name)))
            .and_then(|h| h["value"].as_str())
            .map(str::to_owned)
    };
    let millis: i64 = json["internalDate"].as_str()?.parse().ok()?;
    Some(MailMessage {
        id: json["id"].as_str()?.to_owned(),
        thread_id: json["threadId"].as_str()?.to_owned(),
        from: sender_name(&header("From").unwrap_or_default()),
        subject: header("Subject")
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| "(no subject)".into()),
        snippet: unescape(json["snippet"].as_str().unwrap_or_default()),
        at: Utc.timestamp_millis_opt(millis).single()?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_senders_and_messages() {
        assert_eq!(sender_name("\"Ada Lovelace\" <ada@example.com>"), "Ada Lovelace");
        assert_eq!(sender_name("GitHub <noreply@github.com>"), "GitHub");
        assert_eq!(sender_name("<bob@example.com>"), "bob@example.com");
        assert_eq!(sender_name("carol@example.com"), "carol@example.com");
        let json = serde_json::json!({
            "id": "m1",
            "threadId": "t1",
            "snippet": "Tom &amp; Jerry&#39;s plan",
            "internalDate": "1790000000000",
            "payload": {"headers": [
                {"name": "From", "value": "Tom <tom@example.com>"},
                {"name": "Subject", "value": "Lunch?"}
            ]}
        });
        let m = parse_message(&json).unwrap();
        assert_eq!(m.from, "Tom");
        assert_eq!(m.subject, "Lunch?");
        assert_eq!(m.snippet, "Tom & Jerry's plan");
        assert!(parse_message(&serde_json::json!({"id": "x"})).is_none());
    }

    #[test]
    fn snoozes_hide_mail_until_due() {
        let mut conn = rusqlite::Connection::open_in_memory().unwrap();
        crate::db::migrate_for_tests(&mut conn);
        let now = Utc.timestamp_opt(1_790_000_000, 0).unwrap();
        snooze(&conn, "m1", now + chrono::Duration::hours(1), now).unwrap();
        snooze(&conn, "m2", now + chrono::Duration::hours(3), now).unwrap();
        assert!(snooze(&conn, "m3", now, now).is_err());
        assert!(snooze(&conn, "bad id!", now + chrono::Duration::hours(1), now).is_err());
        assert!(snooze(&conn, "m3", now + chrono::Duration::days(100), now).is_err());

        let (hidden, due) = snoozes(&conn, now).unwrap();
        assert_eq!(hidden.len(), 2);
        assert!(due.is_empty());
        let later = now + chrono::Duration::hours(2);
        let (hidden, due) = snoozes(&conn, later).unwrap();
        assert!(hidden.contains("m2") && !hidden.contains("m1"));
        assert_eq!(due, vec!["m1".to_owned()]);
        // Due once only.
        assert!(snoozes(&conn, later).unwrap().1.is_empty());
    }
}
