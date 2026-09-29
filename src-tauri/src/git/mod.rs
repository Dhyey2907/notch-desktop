//! Git module backend (architecture.md §7 Git, plan.md Phase 8).
//!
//! `cli` runs `git`/`gh` as child processes (argument arrays, never a shell); this file holds
//! the provider-neutral types and the pure parsers for their output, so they're unit-tested
//! without a repository. `service` owns the active repo, the file watcher, and operations.

pub mod cli;
pub mod service;

use std::collections::BTreeMap;

use chrono::{DateTime, Local, NaiveDate, TimeZone, Utc};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct RepoInfo {
    pub path: String,
    /// Folder name, used as the display name.
    pub name: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ChangeCounts {
    pub staged: u32,
    pub modified: u32,
    pub untracked: u32,
    pub conflicted: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct BranchStatus {
    /// `None` when HEAD is detached.
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub branch: Option<String>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub upstream: Option<String>,
    pub ahead: u32,
    pub behind: u32,
    pub changes: ChangeCounts,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Commit {
    pub hash: String,
    pub short: String,
    pub subject: String,
    pub author: String,
    pub time: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct DayCount {
    /// Local calendar date, `YYYY-MM-DD`.
    pub date: String,
    pub count: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(tag = "state", rename_all = "camelCase")]
#[ts(export)]
pub enum GitStatus {
    /// The `git` executable wasn't found.
    GitMissing,
    /// No repository chosen yet.
    NoRepo,
    Ready,
    /// The chosen folder is no longer a readable repository.
    Error {
        message: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum GitOperation {
    Commit,
    Push,
    Pull,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct GitSnapshot {
    pub status: GitStatus,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repo: Option<RepoInfo>,
    pub branch: BranchStatus,
    pub commits: Vec<Commit>,
    /// Your commits per day over the last 26 weeks, oldest first.
    pub activity: Vec<DayCount>,
    pub recent_repos: Vec<RepoInfo>,
    /// An operation in progress, if any.
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub running: Option<GitOperation>,
    pub mock: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct PullRequest {
    pub number: u32,
    pub title: String,
    pub author: String,
    pub is_draft: bool,
    /// `APPROVED`, `CHANGES_REQUESTED`, `REVIEW_REQUIRED`, or empty.
    pub review_decision: String,
    pub url: String,
    pub branch: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum PrUnavailable {
    /// GitHub CLI isn't installed.
    GhMissing,
    /// `gh auth login` hasn't been run.
    NotAuthenticated,
    /// The repo has no GitHub remote.
    NotGithub,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(tag = "state", rename_all = "camelCase")]
#[ts(export)]
pub enum PullRequests {
    Ready { items: Vec<PullRequest> },
    Unavailable { reason: PrUnavailable },
}

pub fn repo_info(path: &std::path::Path) -> RepoInfo {
    RepoInfo {
        path: path.display().to_string(),
        name: path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| path.display().to_string()),
    }
}

/// Parse `git status --porcelain=v2 --branch -z` output.
pub fn parse_status(output: &str) -> BranchStatus {
    let mut status = BranchStatus::default();
    let mut records = output.split('\0').filter(|r| !r.is_empty());
    while let Some(record) = records.next() {
        if let Some(header) = record.strip_prefix("# ") {
            let mut parts = header.splitn(2, ' ');
            match (parts.next(), parts.next()) {
                (Some("branch.head"), Some(head)) if head != "(detached)" => {
                    status.branch = Some(head.to_owned())
                }
                (Some("branch.upstream"), Some(up)) => status.upstream = Some(up.to_owned()),
                (Some("branch.ab"), Some(ab)) => {
                    for part in ab.split(' ') {
                        if let Some(n) = part.strip_prefix('+') {
                            status.ahead = n.parse().unwrap_or(0);
                        } else if let Some(n) = part.strip_prefix('-') {
                            status.behind = n.parse().unwrap_or(0);
                        }
                    }
                }
                _ => {}
            }
            continue;
        }
        match record.as_bytes().first() {
            Some(b'1') | Some(b'2') => {
                let xy = record.get(2..4).unwrap_or("..");
                let (x, y) = (xy.as_bytes()[0], xy.as_bytes()[1]);
                if x != b'.' {
                    status.changes.staged += 1;
                }
                if y != b'.' {
                    status.changes.modified += 1;
                }
                // Renames/copies carry the original path as an extra NUL-separated record.
                if record.starts_with('2') {
                    records.next();
                }
            }
            Some(b'u') => status.changes.conflicted += 1,
            Some(b'?') => status.changes.untracked += 1,
            _ => {}
        }
    }
    status
}

/// Unit separator between `git log` fields; can't appear in subjects or names.
pub const LOG_SEPARATOR: char = '\u{1f}';
/// `--format` for [`parse_log`].
pub const LOG_FORMAT: &str = "%H%x1f%h%x1f%s%x1f%an%x1f%ct";

pub fn parse_log(output: &str) -> Vec<Commit> {
    output
        .lines()
        .filter_map(|line| {
            let mut f = line.split(LOG_SEPARATOR);
            let (hash, short, subject, author, time) =
                (f.next()?, f.next()?, f.next()?, f.next()?, f.next()?);
            Some(Commit {
                hash: hash.to_owned(),
                short: short.to_owned(),
                subject: subject.to_owned(),
                author: author.to_owned(),
                time: Utc.timestamp_opt(time.trim().parse().ok()?, 0).single()?,
            })
        })
        .collect()
}

/// Commits per local day for the `days` days ending on `today` (inclusive), oldest first.
/// `timestamps` are Unix seconds, one per line (`git log --format=%ct`).
pub fn activity(timestamps: &str, today: NaiveDate, days: u32) -> Vec<DayCount> {
    let start = today - chrono::Duration::days(i64::from(days) - 1);
    let mut counts: BTreeMap<NaiveDate, u32> = BTreeMap::new();
    for line in timestamps.lines() {
        let Some(t) = line
            .trim()
            .parse::<i64>()
            .ok()
            .and_then(|s| Local.timestamp_opt(s, 0).single())
        else {
            continue;
        };
        let day = t.date_naive();
        if day >= start && day <= today {
            *counts.entry(day).or_default() += 1;
        }
    }
    (0..days)
        .map(|i| {
            let date = start + chrono::Duration::days(i64::from(i));
            DayCount {
                date: date.format("%Y-%m-%d").to_string(),
                count: counts.get(&date).copied().unwrap_or(0),
            }
        })
        .collect()
}

/// Remove credentials that git can echo in errors, e.g. `https://user:token@github.com/...`.
pub fn redact(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(pos) = rest.find("://") {
        let (head, tail) = rest.split_at(pos + 3);
        out.push_str(head);
        let authority_end = tail.find(['/', ' ', '\n', '\'', '"']).unwrap_or(tail.len());
        match tail[..authority_end].rfind('@') {
            Some(at) => {
                out.push_str("***@");
                rest = &tail[at + 1..];
            }
            None => rest = tail,
        }
    }
    out.push_str(rest);
    out
}

/// A short, safe message for the UI from git's stderr: the first meaningful line, redacted.
pub fn summarize_error(stderr: &str) -> String {
    // Common situations get plain advice instead of git's wording.
    const KNOWN: [(&str, &str); 11] = [
        (
            "not possible to fast-forward",
            "Your branch and the remote have both changed. Rebase or merge in a terminal, then try again.",
        ),
        (
            "fetch first",
            "The remote has commits you don't have yet. Pull first, then push.",
        ),
        (
            "non-fast-forward",
            "The remote has commits you don't have yet. Pull first, then push.",
        ),
        (
            "authentication failed",
            "Git couldn't sign in to the remote. Check your credentials.",
        ),
        (
            "could not read username",
            "Git couldn't sign in to the remote. Check your credentials.",
        ),
        (
            "permission denied",
            "The remote refused access. Check that you can push to this repository.",
        ),
        (
            "could not resolve host",
            "Can't reach the remote. Check your internet connection.",
        ),
        ("nothing to commit", "Nothing to commit."),
        (
            "no configured push destination",
            "This repository has no remote to push to.",
        ),
        (
            "please tell me who you are",
            "Set your Git name and email first (git config user.name / user.email).",
        ),
        (
            "would be overwritten by merge",
            "You have local changes that pulling would overwrite. Commit or stash them first.",
        ),
    ];
    let lower = stderr.to_ascii_lowercase();
    if let Some((_, advice)) = KNOWN.iter().find(|(needle, _)| lower.contains(needle)) {
        return (*advice).to_owned();
    }
    // Otherwise the actual error line; git prints informational lines ("From …", "To …") first.
    let lines: Vec<&str> = stderr.lines().map(str::trim).filter(|l| !l.is_empty()).collect();
    let line = lines
        .iter()
        .find(|l| l.starts_with("fatal:") || l.starts_with("error:"))
        .or_else(|| lines.iter().find(|l| !l.starts_with("hint:")))
        .copied()
        .unwrap_or("git reported an error");
    let line = line.trim_start_matches("fatal: ").trim_start_matches("error: ");
    let mut message = redact(line);
    if message.chars().count() > 160 {
        message = message.chars().take(157).collect::<String>() + "…";
    }
    message
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawPr {
    number: u32,
    #[serde(default)]
    title: String,
    #[serde(default)]
    author: Option<RawAuthor>,
    #[serde(default)]
    is_draft: bool,
    #[serde(default)]
    review_decision: Option<String>,
    #[serde(default)]
    url: String,
    #[serde(default)]
    head_ref_name: String,
}

#[derive(Debug, Deserialize)]
struct RawAuthor {
    #[serde(default)]
    login: String,
}

/// Fields requested from `gh pr list --json`.
pub const PR_FIELDS: &str = "number,title,author,isDraft,reviewDecision,url,headRefName";

/// Parse `gh pr list --json …`; only https links survive (they are opened in the browser).
pub fn parse_prs(json: &str) -> Option<Vec<PullRequest>> {
    let raw: Vec<RawPr> = serde_json::from_str(json).ok()?;
    Some(
        raw.into_iter()
            .filter(|p| p.url.starts_with("https://"))
            .map(|p| PullRequest {
                number: p.number,
                title: p.title,
                author: p.author.map(|a| a.login).unwrap_or_default(),
                is_draft: p.is_draft,
                review_decision: p.review_decision.unwrap_or_default(),
                url: p.url,
                branch: p.head_ref_name,
            })
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_branch_ahead_behind_and_changes() {
        let out = "# branch.oid abc\0# branch.head main\0# branch.upstream origin/main\0# branch.ab +2 -1\0\
                   1 .M N... 100644 100644 100644 a a src/a.rs\0\
                   1 M. N... 100644 100644 100644 a b src/b.rs\0\
                   1 MM N... 100644 100644 100644 a b src/c.rs\0\
                   2 R. N... 100644 100644 100644 a b R100 new.rs\0old.rs\0\
                   u UU N... 100644 100644 100644 100644 a b c conflict.rs\0\
                   ? notes.txt\0? other.txt\0";
        let s = parse_status(out);
        assert_eq!(s.branch.as_deref(), Some("main"));
        assert_eq!(s.upstream.as_deref(), Some("origin/main"));
        assert_eq!((s.ahead, s.behind), (2, 1));
        assert_eq!(
            s.changes,
            ChangeCounts {
                staged: 3,
                modified: 2,
                untracked: 2,
                conflicted: 1
            }
        );
    }

    #[test]
    fn detached_head_and_no_upstream() {
        let s = parse_status("# branch.oid abc\0# branch.head (detached)\0");
        assert_eq!(s.branch, None);
        assert_eq!(s.upstream, None);
        assert_eq!((s.ahead, s.behind), (0, 0));
    }

    #[test]
    fn parses_log_lines() {
        let out = "abcdef1234\u{1f}abcdef1\u{1f}Fix: a | b\u{1f}Dhyey\u{1f}1790000000\nbroken line\n";
        let commits = parse_log(out);
        assert_eq!(commits.len(), 1);
        assert_eq!(commits[0].subject, "Fix: a | b");
        assert_eq!(commits[0].time.timestamp(), 1_790_000_000);
    }

    #[test]
    fn buckets_commits_by_local_day() {
        let today = NaiveDate::from_ymd_opt(2026, 9, 29).unwrap();
        let at = |d: u32, h: u32| Local.with_ymd_and_hms(2026, 9, d, h, 0, 0).unwrap().timestamp();
        let ts = format!(
            "{}\n{}\n{}\n{}\nnot-a-number\n",
            at(29, 10),
            at(29, 23),
            at(28, 9),
            at(1, 9)
        );
        let days = activity(&ts, today, 7);
        assert_eq!(days.len(), 7);
        assert_eq!(days.first().unwrap().date, "2026-09-23");
        assert_eq!(
            days.last().unwrap(),
            &DayCount {
                date: "2026-09-29".into(),
                count: 2
            }
        );
        assert_eq!(days[5].count, 1);
        assert_eq!(days.iter().map(|d| d.count).sum::<u32>(), 3); // Sep 1 is out of range
    }

    #[test]
    fn redacts_credentials_in_urls() {
        assert_eq!(
            redact("fatal: unable to access 'https://user:ghp_secret@github.com/a/b.git/': 403"),
            "fatal: unable to access 'https://***@github.com/a/b.git/': 403"
        );
        assert_eq!(redact("https://github.com/a/b.git"), "https://github.com/a/b.git");
        assert_eq!(redact("git@github.com:a/b.git"), "git@github.com:a/b.git");
    }

    #[test]
    fn summarizes_errors_without_hints_or_secrets() {
        let stderr = "hint: something\nfatal: could not read from 'https://x:tok@host/r'\nmore";
        assert_eq!(
            summarize_error(stderr),
            "could not read from 'https://***@host/r'"
        );
        assert_eq!(summarize_error(""), "git reported an error");
    }

    #[test]
    fn prefers_the_error_line_and_explains_common_failures() {
        let pull = "From C:/remote\n * branch main -> FETCH_HEAD\nhint: Diverging\nfatal: Not possible to fast-forward, aborting.";
        assert!(summarize_error(pull).starts_with("Your branch and the remote have both changed"));
        let push =
            "To C:/remote.git\n ! [rejected]  main -> main (fetch first)\nerror: failed to push some refs";
        assert!(summarize_error(push).starts_with("The remote has commits you don't have yet"));
        assert_eq!(
            summarize_error("To somewhere\nerror: something unusual happened"),
            "something unusual happened"
        );
    }

    #[test]
    fn parses_pull_requests() {
        let json = r#"[
          {"number":214,"title":"Add notch","author":{"login":"dhyey"},"isDraft":false,
           "reviewDecision":"APPROVED","url":"https://github.com/a/b/pull/214","headRefName":"feat/notch"},
          {"number":3,"title":"x","url":"javascript:alert(1)"}
        ]"#;
        let prs = parse_prs(json).unwrap();
        assert_eq!(prs.len(), 1);
        assert_eq!(prs[0].author, "dhyey");
        assert_eq!(prs[0].review_decision, "APPROVED");
        assert!(parse_prs("nope").is_none());
    }
}
