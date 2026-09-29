//! Agent approvals (plan.md Phase 10): answer Claude Code permission requests from the notch.
//!
//! Claude Code's `PermissionRequest` HTTP hook holds its request open while Notch waits for
//! you. Allow / Deny are sent back as the hook's decision. Anything else — dismissing,
//! "answer in terminal", a timeout, or Notch quitting — returns no decision, and Claude Code
//! falls back to its normal terminal prompt. The notch is never the only way to answer, and
//! nothing is ever allowed without an explicit click.

use std::{
    collections::{HashMap, HashSet},
    sync::{mpsc, Mutex},
    time::Duration,
};

use chrono::{DateTime, Utc};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use ts_rs::TS;

use crate::error::AppResult;

/// How long Notch waits for you before handing the prompt back to the terminal. Kept below
/// the hook timeout written into Claude Code's settings so Notch always answers first.
pub const DECISION_WAIT: Duration = Duration::from_secs(110);
/// Hook timeout (seconds) configured for `PermissionRequest`.
pub const HOOK_TIMEOUT_SECS: u64 = 120;
/// Longest command/target shown and stored.
const MAX_SUMMARY: usize = 4000;
/// Audit rows kept.
const AUDIT_KEEP: i64 = 500;

/// Tools that are conversations with you rather than actions: Claude Code shows its own
/// question or plan dialog for them. They're never turned into approval cards.
pub const USER_DIALOG_TOOLS: [&str; 3] = ["AskUserQuestion", "ExitPlanMode", "EnterPlanMode"];

pub fn is_user_dialog(tool: &str) -> bool {
    USER_DIALOG_TOOLS.contains(&tool)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ApprovalRequest {
    pub id: String,
    pub session_id: String,
    pub project: String,
    /// e.g. "Bash", "Write", "WebFetch".
    pub tool: String,
    /// What exactly would happen: the full command, file path, or URL.
    pub summary: String,
    /// Looks destructive (e.g. `rm -rf`, `git push --force`); shown with a stronger warning.
    pub destructive: bool,
    /// Why it was flagged, in plain words.
    pub reasons: Vec<String>,
    pub created_at: DateTime<Utc>,
    /// When Notch gives up and hands the prompt back to the terminal.
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum ApprovalChoice {
    Allow,
    /// Allow now, and auto-allow this exact request for the rest of the session.
    AllowForSession,
    Deny,
    /// Hand the question back to Claude Code's own terminal prompt.
    AnswerInTerminal,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct AuditEntry {
    pub id: String,
    pub at: DateTime<Utc>,
    pub project: String,
    pub tool: String,
    pub summary: String,
    pub destructive: bool,
    /// allow | deny | ask
    pub decision: String,
    /// you | session_rule | timeout | app_closed
    pub decided_by: String,
}

/// Fields of a `PermissionRequest` payload that Notch needs.
#[derive(Debug, Deserialize)]
pub struct PermissionPayload {
    #[serde(default)]
    pub session_id: String,
    #[serde(default)]
    pub cwd: Option<String>,
    #[serde(default)]
    pub tool_name: String,
    #[serde(default)]
    pub tool_input: Value,
}

fn truncate(text: &str, max: usize) -> String {
    if text.chars().count() <= max {
        text.to_owned()
    } else {
        text.chars().take(max).collect::<String>() + "…"
    }
}

/// The exact thing being approved, per tool.
pub fn summarize(tool: &str, input: &Value) -> String {
    let field = |name: &str| input.get(name).and_then(Value::as_str).map(str::to_owned);
    let text = match tool {
        "Bash" | "PowerShell" => field("command"),
        "Write" | "Edit" | "MultiEdit" | "Read" => field("file_path"),
        "NotebookEdit" => field("notebook_path"),
        "WebFetch" => field("url"),
        "WebSearch" => field("query"),
        _ => None,
    };
    let text = text.unwrap_or_else(|| {
        // Unknown tools: show their arguments compactly rather than hiding them.
        let compact = serde_json::to_string(input).unwrap_or_default();
        if compact == "null" || compact == "{}" {
            tool.to_owned()
        } else {
            compact
        }
    });
    truncate(text.trim(), MAX_SUMMARY)
}

/// Flag commands that are hard to undo. A heuristic for extra caution, not a security
/// boundary: it only adds a warning and a second confirmation click.
pub fn classify(tool: &str, summary: &str) -> Vec<String> {
    if !matches!(tool, "Bash" | "PowerShell") {
        return Vec::new();
    }
    // Pad and normalise separators so " rm " also matches at the start, after `;`, `&&`, `|`.
    let cmd = format!(" {} ", summary.to_ascii_lowercase())
        .replace(['\n', '\t', ';', '&', '|', '(', ')'], " ")
        .replace("  ", " ");
    let piped = summary.contains('|');
    let has = |needle: &str| cmd.contains(needle);
    let mut reasons = Vec::new();
    let mut flag = |cond: bool, reason: &str| {
        if cond && !reasons.iter().any(|r: &String| r == reason) {
            reasons.push(reason.to_owned());
        }
    };
    // A short flag cluster like `-rf` / `-fdx` containing `letter`.
    let short_flag = |letter: char| {
        cmd.split_whitespace()
            .any(|t| t.starts_with('-') && !t.starts_with("--") && t.contains(letter))
    };
    flag(
        has(" rm ") && (short_flag('r') || has(" --recursive")),
        "Recursively deletes files",
    );
    flag(
        (has(" del ") || has(" rmdir ") || has(" rd ")) && has(" /s"),
        "Recursively deletes files",
    );
    flag(has("remove-item") && has(" -r"), "Recursively deletes files");
    flag(
        has(" git push") && (has(" --force") || short_flag('f') || has(" +")),
        "Force-pushes, which can overwrite remote history",
    );
    flag(
        has(" git reset") && has(" --hard"),
        "Discards uncommitted changes (git reset --hard)",
    );
    flag(
        has(" git clean") && (short_flag('f') || has(" --force")),
        "Deletes untracked files (git clean)",
    );
    flag(
        has(" git branch") && summary.contains(" -D"),
        "Force-deletes a branch",
    );
    flag(
        has(" git checkout -- .") || has(" git restore ."),
        "Discards local changes",
    );
    flag(
        has("drop table") || has("drop database") || has("truncate table"),
        "Deletes database data",
    );
    flag(
        has(" format ") && (has(" c:") || has(" d:") || has(" /q")),
        "Formats a drive",
    );
    flag(
        has("mkfs") || has(" dd if=") || has("diskpart"),
        "Writes directly to a disk",
    );
    flag(
        has(" reg delete") || has("remove-itemproperty"),
        "Deletes registry data",
    );
    flag(
        has(" shutdown ") || has("stop-computer") || has("restart-computer"),
        "Shuts down or restarts the PC",
    );
    flag(
        piped
            && (has("curl ") || has("wget ") || has("iwr ") || has("invoke-webrequest") || has("irm "))
            && (has(" sh ") || has(" bash ") || has(" iex ") || has(" invoke-expression ")),
        "Downloads and runs a script",
    );
    flag(
        has(" chmod -r 777") || has(" chmod 777 -r"),
        "Opens permissions on a whole tree",
    );
    reasons
}

/// Hook response body for a decision. `None` = no decision (terminal prompt fallback).
pub fn response_body(behavior: Option<&str>) -> String {
    match behavior {
        Some(behavior) => json!({
            "hookSpecificOutput": {
                "hookEventName": "PermissionRequest",
                "decision": { "behavior": behavior }
            }
        })
        .to_string(),
        None => "{}".to_owned(),
    }
}

/// How a waiting request ends.
enum Signal {
    /// You answered in the notch.
    Choice(ApprovalChoice),
    /// Claude Code went away (session ended or process died).
    AgentExited,
    /// You answered in Claude Code itself; the card is no longer needed.
    Elsewhere,
}

type Answer = mpsc::Sender<Signal>;

/// Pending requests and per-session "allow this exact request" rules.
#[derive(Default)]
pub struct Approvals {
    pending: Mutex<HashMap<String, (ApprovalRequest, Answer)>>,
    session_rules: Mutex<HashSet<(String, String, String)>>,
}

/// What happened to a request, for the response and the audit row.
#[derive(Debug, PartialEq, Eq)]
pub struct Outcome {
    pub behavior: Option<&'static str>,
    pub decided_by: &'static str,
}

impl Approvals {
    pub fn pending(&self) -> Vec<ApprovalRequest> {
        let mut list: Vec<ApprovalRequest> = self
            .pending
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .values()
            .map(|(r, _)| r.clone())
            .collect();
        list.sort_by_key(|r| r.created_at);
        list
    }

    /// Build a request from a hook payload.
    pub fn request(payload: &PermissionPayload, now: DateTime<Utc>) -> ApprovalRequest {
        let summary = summarize(&payload.tool_name, &payload.tool_input);
        let reasons = classify(&payload.tool_name, &summary);
        ApprovalRequest {
            id: uuid::Uuid::new_v4().to_string(),
            session_id: payload.session_id.clone(),
            project: payload
                .cwd
                .as_deref()
                .map(|c| {
                    c.trim_end_matches(['\\', '/'])
                        .rsplit(['\\', '/'])
                        .next()
                        .unwrap_or(c)
                        .to_owned()
                })
                .unwrap_or_default(),
            tool: if payload.tool_name.is_empty() {
                "Tool".into()
            } else {
                truncate(&payload.tool_name, 48)
            },
            destructive: !reasons.is_empty(),
            reasons,
            summary,
            created_at: now,
            expires_at: now + chrono::Duration::from_std(DECISION_WAIT).unwrap_or_default(),
        }
    }

    /// True if the user already allowed this exact request for this session.
    pub fn allowed_by_session_rule(&self, request: &ApprovalRequest) -> bool {
        self.session_rules
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .contains(&(
                request.session_id.clone(),
                request.tool.clone(),
                request.summary.clone(),
            ))
    }

    /// Register a pending request and block until the user decides or `wait` passes.
    /// `on_change` is called when the pending list changes (to update the UI).
    pub fn wait_for_decision(
        &self,
        request: ApprovalRequest,
        wait: Duration,
        on_change: &dyn Fn(),
    ) -> Outcome {
        let (tx, rx) = mpsc::channel();
        let id = request.id.clone();
        self.pending
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .insert(id.clone(), (request.clone(), tx));
        on_change();

        let outcome = match rx.recv_timeout(wait) {
            Ok(Signal::Choice(ApprovalChoice::Allow)) => Outcome {
                behavior: Some("allow"),
                decided_by: "you",
            },
            Ok(Signal::Choice(ApprovalChoice::AllowForSession)) => {
                self.session_rules
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .insert((
                        request.session_id.clone(),
                        request.tool.clone(),
                        request.summary.clone(),
                    ));
                Outcome {
                    behavior: Some("allow"),
                    decided_by: "you",
                }
            }
            Ok(Signal::Choice(ApprovalChoice::Deny)) => Outcome {
                behavior: Some("deny"),
                decided_by: "you",
            },
            Ok(Signal::Choice(ApprovalChoice::AnswerInTerminal)) => Outcome {
                behavior: None,
                decided_by: "you",
            },
            Ok(Signal::AgentExited) => Outcome {
                behavior: None,
                decided_by: "agent_exited",
            },
            Ok(Signal::Elsewhere) => Outcome {
                behavior: None,
                decided_by: "elsewhere",
            },
            Err(mpsc::RecvTimeoutError::Timeout) => Outcome {
                behavior: None,
                decided_by: "timeout",
            },
            Err(mpsc::RecvTimeoutError::Disconnected) => Outcome {
                behavior: None,
                decided_by: "app_closed",
            },
        };
        self.pending.lock().unwrap_or_else(|e| e.into_inner()).remove(&id);
        on_change();
        outcome
    }

    /// Deliver the user's choice. Returns false if the request already expired.
    pub fn decide(&self, id: &str, choice: ApprovalChoice) -> bool {
        self.pending
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get(id)
            .is_some_and(|(_, tx)| tx.send(Signal::Choice(choice)).is_ok())
    }

    /// A session ended (or its process died): forget its rules and close its open requests,
    /// since nobody is waiting for those answers any more.
    pub fn end_session(&self, session_id: &str) {
        self.session_rules
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .retain(|(s, _, _)| s != session_id);
        for (request, tx) in self.pending.lock().unwrap_or_else(|e| e.into_inner()).values() {
            if request.session_id == session_id {
                let _ = tx.send(Signal::AgentExited);
            }
        }
    }

    /// Close cards you've already answered in Claude Code. With `tool` (from a `PostToolUse`),
    /// only the matching request closes, so an unrelated tool running alongside doesn't; without
    /// it (the turn moved on: new prompt or `Stop`), every open card of the session closes.
    /// Never allows anything — the hook just returns no decision.
    pub fn resolve_elsewhere(&self, session_id: &str, tool: Option<(&str, &str)>) -> usize {
        let mut closed = 0;
        for (request, tx) in self.pending.lock().unwrap_or_else(|e| e.into_inner()).values() {
            let matches = request.session_id == session_id
                && tool.is_none_or(|(name, summary)| request.tool == name && request.summary == summary);
            if matches && tx.send(Signal::Elsewhere).is_ok() {
                closed += 1;
            }
        }
        closed
    }
}

pub fn record(conn: &Connection, request: &ApprovalRequest, outcome: &Outcome) -> AppResult<()> {
    conn.execute(
        "INSERT INTO approval_audit (id, ts, session_id, project, tool, summary, destructive, decision, decided_by)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        params![
            request.id,
            Utc::now().timestamp(),
            request.session_id,
            request.project,
            request.tool,
            request.summary,
            request.destructive,
            outcome.behavior.unwrap_or("ask"),
            outcome.decided_by,
        ],
    )?;
    conn.execute(
        "DELETE FROM approval_audit WHERE id NOT IN (SELECT id FROM approval_audit ORDER BY ts DESC LIMIT ?1)",
        [AUDIT_KEEP],
    )?;
    Ok(())
}

pub fn recent(conn: &Connection, limit: u32) -> AppResult<Vec<AuditEntry>> {
    let mut stmt = conn.prepare(
        "SELECT id, ts, project, tool, summary, destructive, decision, decided_by
         FROM approval_audit ORDER BY ts DESC LIMIT ?1",
    )?;
    let rows = stmt.query_map([limit.min(200)], |r| {
        Ok(AuditEntry {
            id: r.get(0)?,
            at: DateTime::from_timestamp(r.get(1)?, 0).unwrap_or_default(),
            project: r.get(2)?,
            tool: r.get(3)?,
            summary: r.get(4)?,
            destructive: r.get(5)?,
            decision: r.get(6)?,
            decided_by: r.get(7)?,
        })
    })?;
    Ok(rows.collect::<Result<_, _>>()?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    fn payload(tool: &str, input: Value) -> PermissionPayload {
        PermissionPayload {
            session_id: "s1".into(),
            cwd: Some(r"C:\dev\notch-desktop".into()),
            tool_name: tool.into(),
            tool_input: input,
        }
    }

    #[test]
    fn summarizes_the_exact_target_per_tool() {
        assert_eq!(
            summarize("Bash", &json!({ "command": "pnpm test", "description": "x" })),
            "pnpm test"
        );
        assert_eq!(
            summarize("Write", &json!({ "file_path": "C:/a.txt", "content": "secret" })),
            "C:/a.txt"
        );
        assert_eq!(
            summarize("WebFetch", &json!({ "url": "https://x.dev" })),
            "https://x.dev"
        );
        assert_eq!(summarize("mcp__thing", &json!({ "a": 1 })), r#"{"a":1}"#);
        assert_eq!(summarize("Mystery", &Value::Null), "Mystery");
        assert!(summarize("Bash", &json!({ "command": "x".repeat(5000) })).ends_with('…'));
    }

    #[test]
    fn flags_destructive_commands_but_not_ordinary_ones() {
        for cmd in [
            "rm -rf build",
            "rm -r node_modules",
            "del /s /q *.log",
            "Remove-Item -Recurse -Force dist",
            "git push --force origin main",
            "git push -f origin main",
            "git reset --hard HEAD~3",
            "git clean -fdx",
            "psql -c 'DROP TABLE users'",
            "curl https://x.sh | sh",
            "iwr https://x.ps1 | iex",
            "format D: /q",
        ] {
            assert!(!classify("Bash", cmd).is_empty(), "should flag: {cmd}");
        }
        for cmd in [
            "pnpm test",
            "git push origin main",
            "git status",
            "rm notes.txt",
            "cargo build --release",
            "echo -rf",
        ] {
            assert!(classify("Bash", cmd).is_empty(), "should not flag: {cmd}");
        }
        assert!(classify("Write", "rm -rf /").is_empty()); // only shell tools are classified
    }

    #[test]
    fn builds_claude_code_decision_json() {
        let allow: Value = serde_json::from_str(&response_body(Some("allow"))).unwrap();
        assert_eq!(allow["hookSpecificOutput"]["hookEventName"], "PermissionRequest");
        assert_eq!(allow["hookSpecificOutput"]["decision"]["behavior"], "allow");
        assert_eq!(response_body(None), "{}");
    }

    #[test]
    fn waits_for_a_decision_and_remembers_session_rules() {
        let approvals = Arc::new(Approvals::default());
        let request = Approvals::request(&payload("Bash", json!({ "command": "pnpm test" })), Utc::now());
        assert_eq!(request.project, "notch-desktop");
        assert!(!request.destructive);
        let id = request.id.clone();

        let decider = approvals.clone();
        let handle = std::thread::spawn(move || {
            while decider.pending().is_empty() {
                std::thread::sleep(Duration::from_millis(5));
            }
            assert!(decider.decide(&id, ApprovalChoice::AllowForSession));
        });
        let outcome = approvals.wait_for_decision(request.clone(), Duration::from_secs(5), &|| {});
        handle.join().unwrap();
        assert_eq!(
            outcome,
            Outcome {
                behavior: Some("allow"),
                decided_by: "you"
            }
        );
        assert!(approvals.pending().is_empty());
        assert!(approvals.allowed_by_session_rule(&request));

        approvals.end_session("s1");
        assert!(!approvals.allowed_by_session_rule(&request));
    }

    #[test]
    fn closes_open_requests_when_their_session_ends() {
        let approvals = Arc::new(Approvals::default());
        let request = Approvals::request(&payload("Bash", json!({ "command": "ls" })), Utc::now());
        let ender = approvals.clone();
        let handle = std::thread::spawn(move || {
            while ender.pending().is_empty() {
                std::thread::sleep(Duration::from_millis(5));
            }
            ender.end_session("s1");
        });
        let outcome = approvals.wait_for_decision(request, Duration::from_secs(5), &|| {});
        handle.join().unwrap();
        assert_eq!(
            outcome,
            Outcome {
                behavior: None,
                decided_by: "agent_exited"
            }
        );
        assert!(approvals.pending().is_empty());
    }

    #[test]
    fn questions_and_plans_are_not_approvals() {
        for tool in ["AskUserQuestion", "ExitPlanMode", "EnterPlanMode"] {
            assert!(is_user_dialog(tool));
        }
        for tool in ["Bash", "Write", "WebFetch", "askuserquestion"] {
            assert!(!is_user_dialog(tool));
        }
    }

    #[test]
    fn closes_cards_answered_in_claude_code_but_not_unrelated_ones() {
        let approvals = Arc::new(Approvals::default());
        let wanted = Approvals::request(&payload("Bash", json!({ "command": "pnpm test" })), Utc::now());
        let other = Approvals::request(&payload("Bash", json!({ "command": "ls" })), Utc::now());
        let waiter = |approvals: Arc<Approvals>, request: ApprovalRequest| {
            std::thread::spawn(move || approvals.wait_for_decision(request, Duration::from_secs(5), &|| {}))
        };
        let wanted_thread = waiter(approvals.clone(), wanted);
        let other_thread = waiter(approvals.clone(), other);
        while approvals.pending().len() < 2 {
            std::thread::sleep(Duration::from_millis(5));
        }
        // A parallel tool finishing, and another session, leave the card alone.
        assert_eq!(approvals.resolve_elsewhere("s1", Some(("Read", "C:/a.txt"))), 0);
        assert_eq!(approvals.resolve_elsewhere("s2", None), 0);
        // The same command running means it was answered in Claude Code.
        assert_eq!(approvals.resolve_elsewhere("s1", Some(("Bash", "pnpm test"))), 1);
        let elsewhere = Outcome {
            behavior: None,
            decided_by: "elsewhere",
        };
        assert_eq!(wanted_thread.join().unwrap(), elsewhere);
        // The turn moving on closes whatever is left.
        assert_eq!(approvals.resolve_elsewhere("s1", None), 1);
        assert_eq!(other_thread.join().unwrap(), elsewhere);
        assert!(approvals.pending().is_empty());
    }

    #[test]
    fn times_out_to_the_terminal_prompt() {
        let approvals = Approvals::default();
        let request = Approvals::request(&payload("Bash", json!({ "command": "ls" })), Utc::now());
        let outcome = approvals.wait_for_decision(request, Duration::from_millis(20), &|| {});
        assert_eq!(
            outcome,
            Outcome {
                behavior: None,
                decided_by: "timeout"
            }
        );
        assert!(!approvals.decide("gone", ApprovalChoice::Allow));
    }

    #[test]
    fn audits_decisions_with_the_exact_command() {
        let mut conn = Connection::open_in_memory().unwrap();
        crate::db::migrate_for_tests(&mut conn);
        let request = Approvals::request(&payload("Bash", json!({ "command": "rm -rf build" })), Utc::now());
        record(
            &conn,
            &request,
            &Outcome {
                behavior: Some("deny"),
                decided_by: "you",
            },
        )
        .unwrap();
        let rows = recent(&conn, 10).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].summary, "rm -rf build");
        assert!(rows[0].destructive);
        assert_eq!(
            (rows[0].decision.as_str(), rows[0].decided_by.as_str()),
            ("deny", "you")
        );
    }
}
