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
}
