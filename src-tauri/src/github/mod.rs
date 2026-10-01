//! "Continue with GitHub": device-flow sign-in (show a code, approve it on github.com — the
//! same flow the GitHub CLI uses) and pull requests read straight from GitHub's API.
//!
//! The token lives in Windows Credential Manager. Without a sign-in, the Git module keeps
//! using the `gh` CLI for pull requests.

pub mod notifications;
pub mod service;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Owner and repository name from a GitHub remote URL (https or ssh forms).
pub fn parse_remote(url: &str) -> Option<(String, String)> {
    let url = url.trim();
    let path = url
        .strip_prefix("https://github.com/")
        .or_else(|| url.strip_prefix("http://github.com/"))
        .or_else(|| url.strip_prefix("git@github.com:"))
        .or_else(|| url.strip_prefix("ssh://git@github.com/"))
        .or_else(|| {
            // https://user@github.com/owner/repo
            let rest = url.strip_prefix("https://")?;
            let (_, after) = rest.split_once('@')?;
            after.strip_prefix("github.com/")
        })?;
    let path = path.trim_end_matches('/').trim_end_matches(".git");
    let (owner, name) = path.split_once('/')?;
    let valid = |s: &str| {
        !s.is_empty()
            && s.len() <= 100
            && s != "."
            && s != ".."
            && s.chars().all(|c| c.is_ascii_alphanumeric() || "-_.".contains(c))
    };
    (valid(owner) && valid(name) && !name.contains('/')).then(|| (owner.to_owned(), name.to_owned()))
}

/// `owner/name`, as GitHub spells repositories.
pub fn valid_name_with_owner(value: &str) -> bool {
    parse_remote(&format!("https://github.com/{value}")).is_some_and(|(o, n)| format!("{o}/{n}") == value)
}

/// One of your repositories, for the Git module's picker.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct GithubRepo {
    pub name_with_owner: String,
    #[ts(optional)]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub is_private: bool,
    #[ts(optional)]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pushed_at: Option<DateTime<Utc>>,
    pub url: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct GithubCommit {
    pub headline: String,
    pub author: String,
    pub at: DateTime<Utc>,
    pub url: String,
}

/// A repository shown straight from GitHub (no local clone needed).
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct GithubRepoOverview {
    pub repo: GithubRepo,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_branch: Option<String>,
    /// Latest commits on the default branch.
    pub commits: Vec<GithubCommit>,
    pub pull_requests: crate::git::PullRequests,
}

/// Parse `viewer.repositories.nodes`; only https links survive.
pub fn parse_repos(nodes: &serde_json::Value) -> Vec<GithubRepo> {
    serde_json::from_value::<Vec<GithubRepo>>(nodes.clone())
        .unwrap_or_default()
        .into_iter()
        .filter(|r| r.url.starts_with("https://github.com/") && valid_name_with_owner(&r.name_with_owner))
        .collect()
}

/// Parse `defaultBranchRef.target.history.nodes`.
pub fn parse_commits(nodes: &serde_json::Value) -> Vec<GithubCommit> {
    nodes
        .as_array()
        .map(|list| {
            list.iter()
                .filter_map(|c| {
                    let url = c.get("url")?.as_str()?;
                    if !url.starts_with("https://github.com/") {
                        return None;
                    }
                    let author = c
                        .pointer("/author/user/login")
                        .filter(|v| v.is_string())
                        .or_else(|| c.pointer("/author/name"))
                        .and_then(|a| a.as_str())
                        .unwrap_or("")
                        .to_owned();
                    Some(GithubCommit {
                        headline: c.get("messageHeadline")?.as_str()?.chars().take(200).collect(),
                        author,
                        at: c.get("committedDate")?.as_str()?.parse().ok()?,
                        url: url.to_owned(),
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

/// Shown while waiting for you to approve the sign-in on github.com.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct DeviceCode {
    /// e.g. "WDJB-MJHT" — you type this on github.com/login/device.
    pub user_code: String,
    pub verification_uri: String,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct GithubStatus {
    /// This build has a GitHub client, so "Continue with GitHub" is offered.
    pub available: bool,
    pub signed_in: bool,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub login: Option<String>,
    /// Sign-in in progress.
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pending: Option<DeviceCode>,
    /// Why the last sign-in attempt stopped, in plain words.
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_error: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct DeviceCodeResponse {
    pub device_code: String,
    pub user_code: String,
    pub verification_uri: String,
    pub expires_in: i64,
    #[serde(default = "default_interval")]
    pub interval: u64,
}

fn default_interval() -> u64 {
    5
}

/// One poll of the token endpoint.
#[derive(Debug, PartialEq, Eq)]
pub enum TokenPoll {
    Token(String),
    /// Not approved yet; keep polling (after `extra` more seconds for `slow_down`).
    Pending {
        slow_down: bool,
    },
    Denied,
    Expired,
    Failed(String),
}

pub fn parse_token_poll(json: &serde_json::Value) -> TokenPoll {
    if let Some(token) = json.get("access_token").and_then(|t| t.as_str()) {
        return TokenPoll::Token(token.to_owned());
    }
    match json.get("error").and_then(|e| e.as_str()) {
        Some("authorization_pending") => TokenPoll::Pending { slow_down: false },
        Some("slow_down") => TokenPoll::Pending { slow_down: true },
        Some("access_denied") => TokenPoll::Denied,
        Some("expired_token") => TokenPoll::Expired,
        Some(other) => TokenPoll::Failed(other.to_owned()),
        None => TokenPoll::Failed("unexpected response".into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn reads_owner_and_repo_from_remotes() {
        let expect = Some(("octocat".to_owned(), "Hello-World".to_owned()));
        assert_eq!(parse_remote("https://github.com/octocat/Hello-World.git"), expect);
        assert_eq!(parse_remote("https://github.com/octocat/Hello-World"), expect);
        assert_eq!(parse_remote("git@github.com:octocat/Hello-World.git"), expect);
        assert_eq!(
            parse_remote("ssh://git@github.com/octocat/Hello-World.git"),
            expect
        );
        assert_eq!(
            parse_remote("https://token@github.com/octocat/Hello-World.git"),
            expect
        );
        assert_eq!(parse_remote("https://github.com/octocat/Hello-World/\n"), expect);
    }

    #[test]
    fn ignores_non_github_and_odd_remotes() {
        assert_eq!(parse_remote("https://gitlab.com/a/b.git"), None);
        assert_eq!(parse_remote("C:/dev/notch-shots/gitlab/remote.git"), None);
        assert_eq!(parse_remote("https://github.com/only-owner"), None);
        assert_eq!(parse_remote("https://github.com/a/b/c"), None);
        assert_eq!(parse_remote("https://github.com/a b/c"), None);
    }

    #[test]
    fn validates_repository_names() {
        assert!(valid_name_with_owner("Dhyey2907/notch-desktop"));
        assert!(!valid_name_with_owner("Dhyey2907"));
        assert!(!valid_name_with_owner("a/b/c"));
        assert!(!valid_name_with_owner("a b/c"));
        assert!(!valid_name_with_owner("../etc"));
    }

    #[test]
    fn parses_repos_and_commits_keeping_only_github_links() {
        let repos = parse_repos(&json!([
            { "nameWithOwner": "me/app", "description": "My app", "isPrivate": true,
              "pushedAt": "2026-09-29T10:00:00Z", "url": "https://github.com/me/app" },
            { "nameWithOwner": "me/evil", "isPrivate": false, "url": "javascript:alert(1)" }
        ]));
        assert_eq!(repos.len(), 1);
        assert_eq!(
            (repos[0].name_with_owner.as_str(), repos[0].is_private),
            ("me/app", true)
        );

        let commits = parse_commits(&json!([
            { "messageHeadline": "Fix bug", "committedDate": "2026-09-29T09:00:00Z",
              "url": "https://github.com/me/app/commit/abc", "author": { "name": "Me", "user": { "login": "me" } } },
            { "messageHeadline": "No user", "committedDate": "2026-09-28T09:00:00Z",
              "url": "https://github.com/me/app/commit/def", "author": { "name": "Someone", "user": null } },
            { "messageHeadline": "Bad", "committedDate": "2026-09-28T09:00:00Z", "url": "http://x" }
        ]));
        assert_eq!(commits.len(), 2);
        assert_eq!(
            (commits[0].author.as_str(), commits[1].author.as_str()),
            ("me", "Someone")
        );
    }

    #[test]
    fn understands_token_poll_answers() {
        assert_eq!(
            parse_token_poll(&json!({ "access_token": "gho_x", "token_type": "bearer" })),
            TokenPoll::Token("gho_x".into())
        );
        assert_eq!(
            parse_token_poll(&json!({ "error": "authorization_pending" })),
            TokenPoll::Pending { slow_down: false }
        );
        assert_eq!(
            parse_token_poll(&json!({ "error": "slow_down", "interval": 10 })),
            TokenPoll::Pending { slow_down: true }
        );
        assert_eq!(
            parse_token_poll(&json!({ "error": "access_denied" })),
            TokenPoll::Denied
        );
        assert_eq!(
            parse_token_poll(&json!({ "error": "expired_token" })),
            TokenPoll::Expired
        );
        assert!(matches!(parse_token_poll(&json!({})), TokenPoll::Failed(_)));
    }
}
