//! GitHub notifications (`GET /notifications`): review requests, CI results on runs you
//! started, mentions and assignments, turned into cards in the notch. Titles are shown in the
//! app but never logged.

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::Serialize;
use serde_json::Value;
use ts_rs::TS;

/// Notifications kept for the list (GitHub returns the 50 most recent unread).
pub const MAX_ITEMS: usize = 50;
const MAX_TITLE: usize = 200;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum NotificationKind {
    ReviewRequested,
    CiFailed,
    CiPassed,
    Mention,
    Assigned,
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct GithubNotification {
    /// GitHub's thread id (digits).
    pub id: String,
    pub kind: NotificationKind,
    pub title: String,
    /// `owner/name`.
    pub repo: String,
    /// Web page to open (always https://github.com/…).
    pub url: String,
    pub updated_at: DateTime<Utc>,
}

fn kind_of(reason: &str, subject_type: &str, title: &str) -> NotificationKind {
    if subject_type == "CheckSuite" || reason == "ci_activity" {
        let t = title.to_lowercase();
        return if ["failed", "cancelled", "timed out", "failure"]
            .iter()
            .any(|w| t.contains(w))
        {
            NotificationKind::CiFailed
        } else if ["succeeded", "success", "passed"].iter().any(|w| t.contains(w)) {
            NotificationKind::CiPassed
        } else {
            NotificationKind::Other
        };
    }
    match reason {
        "review_requested" => NotificationKind::ReviewRequested,
        "mention" | "team_mention" => NotificationKind::Mention,
        "assign" => NotificationKind::Assigned,
        _ => NotificationKind::Other,
    }
}

/// API URL of the subject -> its page on github.com. Falls back to the repo's page, or its
/// Actions tab for CI results (those have no subject URL).
fn web_url(repo: &str, subject_type: &str, api_url: Option<&str>) -> String {
    let repo_page = format!("https://github.com/{repo}");
    if subject_type == "CheckSuite" {
        return format!("{repo_page}/actions");
    }
    let prefix = format!("https://api.github.com/repos/{repo}/");
    let Some(rest) = api_url.and_then(|u| u.strip_prefix(&prefix)) else {
        return repo_page;
    };
    let mut parts = rest.split('/');
    match (parts.next(), parts.next(), parts.next()) {
        (Some("pulls"), Some(n), None) if n.chars().all(|c| c.is_ascii_digit()) => {
            format!("{repo_page}/pull/{n}")
        }
        (Some("issues"), Some(n), None) if n.chars().all(|c| c.is_ascii_digit()) => {
            format!("{repo_page}/issues/{n}")
        }
        (Some("discussions"), Some(n), None) if n.chars().all(|c| c.is_ascii_digit()) => {
            format!("{repo_page}/discussions/{n}")
        }
        (Some("commits"), Some(sha), None) if sha.chars().all(|c| c.is_ascii_hexdigit()) => {
            format!("{repo_page}/commit/{sha}")
        }
        (Some("releases"), Some(_), None) => format!("{repo_page}/releases"),
        _ => repo_page,
    }
}

/// Parse the notifications list; malformed entries are skipped.
pub fn parse(json: &Value) -> Vec<GithubNotification> {
    let Some(items) = json.as_array() else {
        return Vec::new();
    };
    items
        .iter()
        .filter_map(|n| {
            let id = n.get("id")?.as_str()?;
            if id.is_empty() || id.len() > 32 || !id.chars().all(|c| c.is_ascii_digit()) {
                return None;
            }
            let repo = n.pointer("/repository/full_name")?.as_str()?;
            if !super::valid_name_with_owner(repo) {
                return None;
            }
            let subject_type = n.pointer("/subject/type").and_then(Value::as_str).unwrap_or("");
            let title: String = n
                .pointer("/subject/title")?
                .as_str()?
                .chars()
                .take(MAX_TITLE)
                .collect();
            let reason = n.get("reason").and_then(Value::as_str).unwrap_or("");
            let updated_at = n.get("updated_at")?.as_str()?.parse().ok()?;
            Some(GithubNotification {
                id: id.to_owned(),
                kind: kind_of(reason, subject_type, &title),
                url: web_url(
                    repo,
                    subject_type,
                    n.pointer("/subject/url").and_then(Value::as_str),
                ),
                repo: repo.to_owned(),
                title,
                updated_at,
            })
        })
        .take(MAX_ITEMS)
        .collect()
}

/// Remembers what was already shown, so only new (or newly updated) notifications pop up.
#[derive(Default)]
pub struct Seen {
    at: HashMap<String, DateTime<Utc>>,
    primed: bool,
}

impl Seen {
    /// Notifications to announce. The first list after signing in only primes the memory:
    /// what was already waiting shows in the list, not as cards.
    pub fn fresh(&mut self, items: &[GithubNotification]) -> Vec<GithubNotification> {
        let fresh: Vec<GithubNotification> = items
            .iter()
            .filter(|n| self.at.get(&n.id).is_none_or(|seen| n.updated_at > *seen))
            .cloned()
            .collect();
        for n in items {
            self.at.insert(n.id.clone(), n.updated_at);
        }
        if self.at.len() > MAX_ITEMS * 4 {
            let keep: std::collections::HashSet<&String> = items.iter().map(|n| &n.id).collect();
            self.at.retain(|id, _| keep.contains(id));
        }
        if std::mem::replace(&mut self.primed, true) {
            fresh
        } else {
            Vec::new()
        }
    }

    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn item(id: &str, reason: &str, kind: &str, title: &str, url: Option<&str>, at: &str) -> Value {
        json!({
            "id": id,
            "reason": reason,
            "updated_at": at,
            "repository": { "full_name": "octo/app" },
            "subject": { "type": kind, "title": title, "url": url },
        })
    }

    #[test]
    fn classifies_and_links_notifications() {
        let list = parse(&json!([
            item(
                "1",
                "review_requested",
                "PullRequest",
                "Add login",
                Some("https://api.github.com/repos/octo/app/pulls/12"),
                "2026-10-01T10:00:00Z"
            ),
            item(
                "2",
                "ci_activity",
                "CheckSuite",
                "CI workflow run failed for main branch",
                None,
                "2026-10-01T10:01:00Z"
            ),
            item(
                "3",
                "ci_activity",
                "CheckSuite",
                "Deploy workflow run succeeded for main branch",
                None,
                "2026-10-01T10:02:00Z"
            ),
            item(
                "4",
                "mention",
                "Issue",
                "Crash on start",
                Some("https://api.github.com/repos/octo/app/issues/7"),
                "2026-10-01T10:03:00Z"
            ),
            item(
                "5",
                "assign",
                "Issue",
                "Fix it",
                Some("https://api.github.com/repos/octo/app/issues/8"),
                "2026-10-01T10:04:00Z"
            ),
            item(
                "6",
                "subscribed",
                "Release",
                "v1.0",
                Some("https://api.github.com/repos/octo/app/releases/99"),
                "2026-10-01T10:05:00Z"
            ),
        ]));
        let kinds: Vec<_> = list.iter().map(|n| n.kind).collect();
        assert_eq!(
            kinds,
            [
                NotificationKind::ReviewRequested,
                NotificationKind::CiFailed,
                NotificationKind::CiPassed,
                NotificationKind::Mention,
                NotificationKind::Assigned,
                NotificationKind::Other
            ]
        );
        assert_eq!(list[0].url, "https://github.com/octo/app/pull/12");
        assert_eq!(list[1].url, "https://github.com/octo/app/actions");
        assert_eq!(list[3].url, "https://github.com/octo/app/issues/7");
        assert_eq!(list[5].url, "https://github.com/octo/app/releases");
    }

    #[test]
    fn never_links_outside_the_repo() {
        let list = parse(&json!([
            item(
                "1",
                "mention",
                "Issue",
                "x",
                Some("https://evil.example/repos/octo/app/issues/1"),
                "2026-10-01T10:00:00Z"
            ),
            item(
                "2",
                "mention",
                "Issue",
                "x",
                Some("https://api.github.com/repos/octo/app/issues/1/../../x"),
                "2026-10-01T10:00:00Z"
            ),
        ]));
        assert!(list.iter().all(|n| n.url == "https://github.com/octo/app"));
    }

    #[test]
    fn skips_malformed_entries() {
        let mut bad_repo = item("1", "mention", "Issue", "x", None, "2026-10-01T10:00:00Z");
        bad_repo["repository"]["full_name"] = json!("../etc");
        let list = parse(&json!([
            bad_repo,
            item("abc", "mention", "Issue", "x", None, "2026-10-01T10:00:00Z"),
            item("3", "mention", "Issue", "x", None, "not a date"),
            json!({ "id": "4" }),
        ]));
        assert!(list.is_empty());
        assert!(parse(&json!({ "message": "Bad credentials" })).is_empty());
    }

    #[test]
    fn announces_only_new_or_updated_notifications() {
        let a = parse(&json!([item(
            "1",
            "mention",
            "Issue",
            "a",
            None,
            "2026-10-01T10:00:00Z"
        )]));
        let mut seen = Seen::default();
        assert!(seen.fresh(&a).is_empty(), "the first list only primes");
        assert!(seen.fresh(&a).is_empty(), "nothing changed");
        let b = parse(&json!([
            item("1", "mention", "Issue", "a", None, "2026-10-01T10:00:00Z"),
            item(
                "2",
                "review_requested",
                "PullRequest",
                "b",
                None,
                "2026-10-01T11:00:00Z"
            ),
        ]));
        assert_eq!(
            seen.fresh(&b).iter().map(|n| n.id.as_str()).collect::<Vec<_>>(),
            ["2"]
        );
        // A CI re-run updates the same thread: announce it again.
        let c = parse(&json!([item(
            "2",
            "review_requested",
            "PullRequest",
            "b",
            None,
            "2026-10-01T12:00:00Z"
        )]));
        assert_eq!(seen.fresh(&c).len(), 1);
        seen.reset();
        assert!(seen.fresh(&c).is_empty(), "primes again after signing out and in");
    }
}
