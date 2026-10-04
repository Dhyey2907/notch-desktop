//! Codex usage, read from the session logs Codex keeps on this PC
//! (`~/.codex/sessions/**/rollout-*.jsonl` and `archived_sessions`). Only counts, models,
//! times and the plan-limit numbers are read — never prompts, answers or code.
//! Files are parsed once and re-read only when they change.

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::Mutex,
    time::SystemTime,
};

use chrono::{DateTime, Duration, Local, TimeZone, Timelike, Utc};
use serde::Serialize;
use serde_json::Value;
use ts_rs::TS;

use super::usage::{DayTokens, ModelUsage, UsageRange, UsageSummary, DAILY_DAYS};

/// Codex's plan limit as last reported in its logs.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct CodexLimit {
    pub used_percent: f64,
    /// Length of the limit's window, e.g. 300 (5 hours) or 10080 (a week).
    pub window_minutes: u32,
    pub resets_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct CodexUsage {
    /// Codex session logs were found on this PC.
    pub found: bool,
    pub summary: UsageSummary,
    pub limits: Vec<CodexLimit>,
}

#[derive(Debug, Clone, Default, PartialEq)]
struct Turn {
    at: DateTime<Utc>,
    model: String,
    input: u64,
    cached: u64,
    output: u64,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Session {
    id: String,
    cwd: String,
    turns: Vec<Turn>,
    prompts: Vec<DateTime<Utc>>,
    /// Latest plan limits seen in this session, with when.
    limits: Option<(DateTime<Utc>, Vec<CodexLimit>)>,
}

fn u(v: &Value) -> u64 {
    v.as_u64().unwrap_or(0)
}

fn limit_of(v: &Value) -> Option<CodexLimit> {
    let used = v.get("used_percent")?.as_f64()?;
    let resets = v.get("resets_at")?.as_i64()?;
    Some(CodexLimit {
        used_percent: used,
        window_minutes: v.get("window_minutes").and_then(Value::as_u64).unwrap_or(0) as u32,
        resets_at: Utc.timestamp_opt(resets, 0).single()?,
    })
}

/// Read one session log (one JSON object per line).
pub fn parse(text: &str) -> Session {
    let mut session = Session::default();
    let mut model = String::new();
    for line in text.lines() {
        let Ok(o) = serde_json::from_str::<Value>(line) else {
            continue;
        };
        let at = o
            .get("timestamp")
            .and_then(Value::as_str)
            .and_then(|t| DateTime::parse_from_rfc3339(t).ok())
            .map(|t| t.with_timezone(&Utc));
        let p = &o["payload"];
        match (o["type"].as_str(), p["type"].as_str()) {
            (Some("session_meta"), _) => {
                session.id = p["id"].as_str().unwrap_or_default().to_owned();
                session.cwd = p["cwd"].as_str().unwrap_or_default().to_owned();
            }
            (Some("turn_context"), _) => {
                if let Some(m) = p["model"].as_str() {
                    model = m.to_owned();
                }
            }
            // One record per model response: what that turn used.
            (Some("token_usage_record"), _) => {
                let usage = &p["usage"];
                if let Some(at) = at {
                    session.turns.push(Turn {
                        at,
                        model: model.clone(),
                        input: u(&usage["input_tokens"]).saturating_sub(u(&usage["cached_input_tokens"])),
                        cached: u(&usage["cached_input_tokens"]),
                        output: u(&usage["output_tokens"]),
                    });
                }
            }
            (Some("event_msg"), Some("token_count")) => {
                let rl = &p["rate_limits"];
                let limits: Vec<CodexLimit> = ["primary", "secondary"]
                    .iter()
                    .filter_map(|k| limit_of(&rl[*k]))
                    .collect();
                if let (Some(at), false) = (at, limits.is_empty()) {
                    session.limits = Some((at, limits));
                }
            }
            // What you typed (Codex's own injected context starts with a tag).
            (Some("response_item"), Some("message")) if p["role"] == "user" => {
                let text = p["content"][0]["text"].as_str().unwrap_or("").trim_start();
                if !text.is_empty() && !text.starts_with('<') {
                    if let Some(at) = at {
                        session.prompts.push(at);
                    }
                }
            }
            _ => {}
        }
    }
    session
}

fn folder_name(cwd: &str) -> String {
    cwd.trim_end_matches(['\\', '/'])
        .rsplit(['\\', '/'])
        .next()
        .unwrap_or(cwd)
        .to_owned()
}

/// The usage summary for a range, in the same shape as Claude Code's.
pub fn summarize(sessions: &[Session], range: UsageRange, now: DateTime<Utc>) -> UsageSummary {
    let since = match range {
        UsageRange::All => DateTime::<Utc>::MIN_UTC,
        UsageRange::Days30 => now - Duration::days(30),
        UsageRange::Days7 => now - Duration::days(7),
    };
    let (mut input, mut cached, mut output) = (0u64, 0u64, 0u64);
    let mut messages = 0u32;
    let mut session_count = 0u32;
    let mut days = std::collections::HashSet::new();
    let mut hours = [0u32; 24];
    let mut models: HashMap<String, (u32, u64, u64)> = HashMap::new();
    let mut projects: HashMap<String, u32> = HashMap::new();
    for s in sessions {
        let turns: Vec<&Turn> = s.turns.iter().filter(|t| t.at >= since).collect();
        let prompts = s.prompts.iter().filter(|p| **p >= since).count() as u32;
        if turns.is_empty() && prompts == 0 {
            continue;
        }
        session_count += 1;
        messages += prompts;
        for t in &turns {
            input += t.input;
            cached += t.cached;
            output += t.output;
            let local = t.at.with_timezone(&Local);
            days.insert(local.date_naive());
            hours[local.hour() as usize] += 1;
            let m = models
                .entry(if t.model.is_empty() {
                    "codex".into()
                } else {
                    t.model.clone()
                })
                .or_default();
            m.0 += 1;
            m.1 += t.input + t.cached + t.output;
            m.2 += t.output;
        }
        if !s.cwd.is_empty() {
            *projects.entry(folder_name(&s.cwd)).or_default() += turns.len() as u32;
        }
    }
    let mut model_list: Vec<ModelUsage> = models
        .into_iter()
        .map(|(model, (messages, tokens, output_tokens))| ModelUsage {
            model,
            messages,
            tokens,
            output_tokens,
        })
        .collect();
    model_list.sort_by_key(|m| std::cmp::Reverse(m.tokens));
    let favorite_model = model_list
        .iter()
        .max_by_key(|m| m.messages)
        .map(|m| m.model.clone());
    let peak_hour = (hours.iter().any(|h| *h > 0)).then(|| {
        (0..24u32)
            .max_by_key(|h| (hours[*h as usize], std::cmp::Reverse(*h)))
            .unwrap_or(0)
    });
    let previous_period_tokens = match range {
        UsageRange::All => None,
        UsageRange::Days30 | UsageRange::Days7 => {
            let span = now - since;
            let start = since - span;
            Some(
                sessions
                    .iter()
                    .flat_map(|s| &s.turns)
                    .filter(|t| t.at >= start && t.at < since)
                    .map(|t| t.input + t.cached + t.output)
                    .sum(),
            )
        }
    };
    let today = now.with_timezone(&Local).date_naive();
    let first_day = today - Duration::days(DAILY_DAYS - 1);
    let mut by_day: HashMap<chrono::NaiveDate, u64> = HashMap::new();
    for t in sessions.iter().flat_map(|s| &s.turns) {
        let day = t.at.with_timezone(&Local).date_naive();
        if day >= first_day {
            *by_day.entry(day).or_default() += t.input + t.cached + t.output;
        }
    }
    let daily = (0..DAILY_DAYS)
        .map(|i| {
            let day = first_day + Duration::days(i);
            DayTokens {
                date: day.format("%Y-%m-%d").to_string(),
                tokens: by_day.get(&day).copied().unwrap_or(0),
            }
        })
        .collect();
    UsageSummary {
        range,
        sessions: session_count,
        messages,
        total_tokens: input + cached + output,
        input_tokens: input,
        output_tokens: output,
        cache_read_tokens: cached,
        cache_creation_tokens: 0,
        active_days: days.len() as u32,
        peak_hour,
        favorite_model,
        models: model_list,
        previous_period_tokens,
        daily,
        top_project: projects.into_iter().max_by_key(|(_, n)| *n).map(|(p, _)| p),
    }
}

/// The newest plan limits across sessions, while they still apply.
pub fn current_limits(sessions: &[Session], now: DateTime<Utc>) -> Vec<CodexLimit> {
    sessions
        .iter()
        .filter_map(|s| s.limits.as_ref())
        .max_by_key(|(at, _)| *at)
        .map(|(_, limits)| limits.iter().filter(|l| l.resets_at > now).cloned().collect())
        .unwrap_or_default()
}

/// Every session log under Codex's folder.
fn session_files(codex_home: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![codex_home.join("sessions"), codex_home.join("archived_sessions")];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for e in entries.flatten() {
            let path = e.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().is_some_and(|x| x == "jsonl") {
                out.push(path);
            }
        }
    }
    out
}

/// Parsed logs, re-read only when a file's size or time changes.
#[derive(Default)]
pub struct CodexIndex {
    files: Mutex<HashMap<PathBuf, (SystemTime, u64, Session)>>,
}

impl CodexIndex {
    pub fn usage(&self, codex_home: &Path, range: UsageRange, now: DateTime<Utc>) -> CodexUsage {
        let paths = session_files(codex_home);
        let mut files = self.files.lock().unwrap_or_else(|e| e.into_inner());
        files.retain(|p, _| paths.contains(p));
        for path in &paths {
            let Ok(meta) = std::fs::metadata(path) else {
                continue;
            };
            let stamp = (meta.modified().unwrap_or(SystemTime::UNIX_EPOCH), meta.len());
            if files.get(path).is_some_and(|(t, l, _)| (*t, *l) == stamp) {
                continue;
            }
            let Ok(text) = std::fs::read_to_string(path) else {
                continue;
            };
            files.insert(path.clone(), (stamp.0, stamp.1, parse(&text)));
        }
        let sessions: Vec<Session> = files.values().map(|(_, _, s)| s.clone()).collect();
        CodexUsage {
            found: codex_home.join("sessions").is_dir() || codex_home.join("archived_sessions").is_dir(),
            summary: summarize(&sessions, range, now),
            limits: current_limits(&sessions, now),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn log(day: &str, model: &str, input: u64, cached: u64, output: u64, used: f64) -> String {
        [
            format!(r#"{{"timestamp":"{day}T10:00:00Z","type":"session_meta","payload":{{"id":"s-{day}","cwd":"C:\\dev\\proj"}}}}"#),
            format!(r#"{{"timestamp":"{day}T10:00:01Z","type":"turn_context","payload":{{"model":"{model}"}}}}"#),
            format!(r#"{{"timestamp":"{day}T10:00:02Z","type":"response_item","payload":{{"type":"message","role":"user","content":[{{"type":"input_text","text":"<environment_context>x</environment_context>"}}]}}}}"#),
            format!(r#"{{"timestamp":"{day}T10:00:03Z","type":"response_item","payload":{{"type":"message","role":"user","content":[{{"type":"input_text","text":"fix the bug"}}]}}}}"#),
            format!(r#"{{"timestamp":"{day}T10:00:09Z","type":"token_usage_record","payload":{{"usage":{{"input_tokens":{input},"cached_input_tokens":{cached},"output_tokens":{output},"total_tokens":0}}}}}}"#),
            format!(r#"{{"timestamp":"{day}T10:00:09Z","type":"event_msg","payload":{{"type":"token_count","rate_limits":{{"primary":{{"used_percent":{used},"window_minutes":300,"resets_at":4102444800}},"secondary":null}}}}}}"#),
            "not json".to_owned(),
        ]
        .join("\n")
    }

    #[test]
    fn reads_turns_prompts_and_limits() {
        let s = parse(&log("2026-10-01", "gpt-x", 1000, 600, 50, 29.0));
        assert_eq!(s.cwd, r"C:\dev\proj");
        assert_eq!(s.prompts.len(), 1, "injected context isn't a prompt");
        assert_eq!(s.turns.len(), 1);
        assert_eq!(
            (s.turns[0].input, s.turns[0].cached, s.turns[0].output),
            (400, 600, 50)
        );
        assert_eq!(s.turns[0].model, "gpt-x");
        let (_, limits) = s.limits.unwrap();
        assert_eq!(limits[0].used_percent, 29.0);
        assert_eq!(limits[0].window_minutes, 300);
    }

    #[test]
    fn summarizes_like_claude() {
        let now = DateTime::parse_from_rfc3339("2026-10-04T12:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let sessions = vec![
            parse(&log("2026-10-01", "gpt-x", 1000, 600, 50, 20.0)),
            parse(&log("2026-10-03", "gpt-y", 300, 0, 10, 35.0)),
            parse(&log("2026-08-01", "gpt-x", 9999, 0, 1, 5.0)),
        ];
        let week = summarize(&sessions, UsageRange::Days7, now);
        assert_eq!(week.sessions, 2);
        assert_eq!(week.messages, 2);
        assert_eq!(week.total_tokens, 1050 + 310);
        assert_eq!(week.active_days, 2);
        assert_eq!(week.models.len(), 2);
        assert_eq!(week.top_project.as_deref(), Some("proj"));
        assert_eq!(week.previous_period_tokens, Some(0));
        assert_eq!(week.daily.len() as i64, DAILY_DAYS);
        let all = summarize(&sessions, UsageRange::All, now);
        assert_eq!(all.sessions, 3);
        // The newest report wins.
        assert_eq!(current_limits(&sessions, now)[0].used_percent, 35.0);
    }
}
