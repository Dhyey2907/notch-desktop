//! "Ask your PC": a question like "the PDF from HDFC last week" or "when did Rahul mail about
//! the invoice?" is turned (by the AI) into searches that run here: file names in your usual
//! folders and recent files, your notes, and, only if you allowed Google data for AI, Gmail
//! and your calendar. Then the AI answers from what was found, citing it. File contents are
//! never read or sent; only names, folders and dates.

use std::path::{Path, PathBuf};

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

pub const PLAN_SYSTEM: &str = "You turn a question about the user's own PC into searches. Reply with \
JSON only, no prose: {\"words\": [up to 5 short keywords for file names and notes], \"days\": number of \
days back to look (0 = any time), \"mail\": a Gmail search query or null, \"calendar\": {\"from\": \
\"YYYY-MM-DD\", \"to\": \"YYYY-MM-DD\"} or null}. Use mail only when the question is about email, \
calendar only when it's about meetings or events.";

pub const ANSWER_SYSTEM: &str = "You answer a question about the user's own PC from the numbered \
results found on it. Be brief (1-4 sentences). Cite results like [2]. If nothing fits, say what \
you looked for and that you didn't find it. Never invent files, mails or events.";

/// What the AI asked to search.
#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
pub struct Plan {
    #[serde(default)]
    pub words: Vec<String>,
    #[serde(default)]
    pub days: u32,
    #[serde(default)]
    pub mail: Option<String>,
    #[serde(default)]
    pub calendar: Option<Range>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Range {
    pub from: String,
    pub to: String,
}

impl Range {
    pub fn dates(&self) -> Option<(NaiveDate, NaiveDate)> {
        let from = NaiveDate::parse_from_str(&self.from, "%Y-%m-%d").ok()?;
        let to = NaiveDate::parse_from_str(&self.to, "%Y-%m-%d").ok()?;
        Some((from.min(to), from.max(to)))
    }
}

/// One thing found, shown under the answer with its number.
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Found {
    /// "file", "note", "mail", "event".
    pub kind: String,
    pub title: String,
    /// Folder, sender, time…
    pub detail: String,
    /// A file to open, when it's a file.
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct PcAnswer {
    pub answer: String,
    pub found: Vec<Found>,
    /// Gmail and the calendar were left out (Google data isn't allowed for AI).
    pub skipped_google: bool,
}

/// The plan from the AI's reply: the first JSON object in it, with sane limits.
pub fn parse_plan(reply: &str) -> Plan {
    let json = reply
        .find('{')
        .and_then(|start| reply.rfind('}').map(|end| &reply[start..=end]))
        .unwrap_or("{}");
    let mut plan: Plan = serde_json::from_str(json).unwrap_or_default();
    plan.words = plan
        .words
        .into_iter()
        .map(|w| w.trim().to_lowercase())
        .filter(|w| w.chars().count() >= 2 && w.len() <= 40)
        .take(5)
        .collect();
    plan.days = plan.days.min(3650);
    plan.mail = plan
        .mail
        .map(|q| q.trim().chars().take(200).collect())
        .filter(|q: &String| !q.is_empty());
    plan
}

/// How well a name matches the words: how many of them it contains (0 = no match).
pub fn score(name: &str, words: &[String]) -> usize {
    let name = name.to_lowercase();
    words.iter().filter(|w| name.contains(w.as_str())).count()
}

/// File names under `roots` (a few levels deep) matching the words, changed within `days`
/// (0 = any time), best first.
pub fn search_files(
    roots: &[PathBuf],
    extra: &[PathBuf],
    words: &[String],
    days: u32,
    now: DateTime<Utc>,
) -> Vec<(PathBuf, DateTime<Utc>, usize)> {
    const MAX_DEPTH: usize = 4;
    const MAX_SEEN: usize = 20_000;
    if words.is_empty() {
        return Vec::new();
    }
    let cutoff = (days > 0).then(|| now - chrono::Duration::days(i64::from(days)));
    let mut found = Vec::new();
    let mut seen = 0usize;
    let consider = |path: &Path, found: &mut Vec<(PathBuf, DateTime<Utc>, usize)>| {
        let Some(name) = path.file_name().map(|n| n.to_string_lossy().into_owned()) else {
            return;
        };
        let s = score(&name, words);
        if s == 0 {
            return;
        }
        let modified: DateTime<Utc> = std::fs::metadata(path)
            .and_then(|m| m.modified())
            .map(DateTime::<Utc>::from)
            .unwrap_or(now);
        if cutoff.is_some_and(|c| modified < c) {
            return;
        }
        if !found.iter().any(|(p, _, _)| p == path) {
            found.push((path.to_path_buf(), modified, s));
        }
    };
    for p in extra {
        consider(p, &mut found);
    }
    // Breadth-first, in the order given, each folder once (home holds the others too).
    let mut queue: std::collections::VecDeque<(PathBuf, usize)> =
        roots.iter().map(|r| (r.clone(), 0)).collect();
    let mut walked = std::collections::HashSet::new();
    while let Some((dir, depth)) = queue.pop_front() {
        if !walked.insert(dir.to_string_lossy().to_lowercase()) {
            continue;
        }
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            seen += 1;
            if seen > MAX_SEEN {
                break;
            }
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().into_owned();
            if name.starts_with('.')
                || ["node_modules", "appdata", "target", "$recycle.bin"]
                    .contains(&name.to_lowercase().as_str())
            {
                continue;
            }
            match entry.file_type() {
                Ok(t) if t.is_dir() => {
                    consider(&path, &mut found);
                    if depth + 1 < MAX_DEPTH {
                        queue.push_back((path, depth + 1));
                    }
                }
                Ok(t) if t.is_file() => consider(&path, &mut found),
                _ => {}
            }
        }
    }
    found.sort_by(|a, b| b.2.cmp(&a.2).then(b.1.cmp(&a.1)));
    found.truncate(12);
    found
}

/// The numbered results as the AI sees them.
pub fn context(question: &str, found: &[Found], today: &str) -> String {
    let mut out = format!("Today is {today}.\nQuestion: {question}\n\nResults found on the PC:\n");
    if found.is_empty() {
        out.push_str("(nothing)\n");
    }
    for (i, f) in found.iter().enumerate() {
        out.push_str(&format!("[{}] {}: {} ({})\n", i + 1, f.kind, f.title, f.detail));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_plan_even_with_prose_around_it() {
        let plan = parse_plan("Sure! {\"words\": [\"HDFC\", \"statement\", \"x\"], \"days\": 7, \"mail\": \"from:hdfc\", \"calendar\": null} hope that helps");
        assert_eq!(plan.words, vec!["hdfc", "statement"]);
        assert_eq!(plan.days, 7);
        assert_eq!(plan.mail.as_deref(), Some("from:hdfc"));
        assert!(plan.calendar.is_none());
        assert_eq!(parse_plan("no json here"), Plan::default());
        let r = parse_plan("{\"calendar\": {\"from\": \"2026-10-20\", \"to\": \"2026-10-18\"}}")
            .calendar
            .unwrap();
        assert_eq!(r.dates().unwrap().0.to_string(), "2026-10-18");
    }

    #[test]
    fn finds_files_by_name_and_date() {
        let dir = std::env::temp_dir().join(format!("overline-askpc-{}", std::process::id()));
        let sub = dir.join("Bank");
        std::fs::create_dir_all(&sub).unwrap();
        std::fs::write(sub.join("HDFC statement Sept.pdf"), b"x").unwrap();
        std::fs::write(dir.join("holiday photo.jpg"), b"x").unwrap();
        let words = vec!["hdfc".to_string(), "statement".to_string()];
        let found = search_files(std::slice::from_ref(&dir), &[], &words, 30, Utc::now());
        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!(found.len(), 1);
        assert!(found[0].0.ends_with("HDFC statement Sept.pdf"));
        assert_eq!(found[0].2, 2);
    }

    #[test]
    fn numbers_the_results_for_the_ai() {
        let f = Found {
            kind: "file".into(),
            title: "a.pdf".into(),
            detail: "Downloads".into(),
            path: None,
        };
        let c = context("where is a", &[f], "2026-10-18");
        assert!(c.contains("[1] file: a.pdf (Downloads)"));
        assert!(context("q", &[], "d").contains("(nothing)"));
    }
}
