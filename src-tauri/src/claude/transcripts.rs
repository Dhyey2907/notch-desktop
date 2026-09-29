//! Incremental indexer for Claude Code transcripts (`~/.claude/projects/**/*.jsonl`).
//!
//! Only counts and metadata are extracted: token usage and model per assistant message,
//! timestamps of typed prompts, and session → working directory. Message text is never
//! stored or logged; prompt content isn't even deserialized (it stays a raw JSON slice that
//! is only checked for "is this a tool result?").

use std::{
    fs::File,
    io::{BufRead, BufReader, Seek, SeekFrom},
    path::{Path, PathBuf},
};

use chrono::{DateTime, Local, Timelike, Utc};
use rusqlite::{params, Connection};
use serde::Deserialize;
use serde_json::value::RawValue;

use crate::error::AppResult;

#[derive(Debug, Deserialize)]
struct Line<'a> {
    #[serde(rename = "type")]
    kind: &'a str,
    #[serde(rename = "sessionId", default)]
    session_id: Option<String>,
    #[serde(default)]
    timestamp: Option<DateTime<Utc>>,
    #[serde(default)]
    uuid: Option<String>,
    #[serde(rename = "isMeta", default)]
    is_meta: Option<bool>,
    #[serde(default)]
    cwd: Option<String>,
    #[serde(borrow, default)]
    message: Option<Message<'a>>,
    #[serde(rename = "quotaLimits", default)]
    quota_limits: Option<QuotaLimits>,
}

#[derive(Debug, Deserialize)]
struct Message<'a> {
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    model: Option<String>,
    #[serde(default)]
    role: Option<&'a str>,
    #[serde(default)]
    usage: Option<Usage>,
    /// Kept raw: prompt text is never parsed or stored.
    #[serde(borrow, default)]
    content: Option<&'a RawValue>,
}

#[derive(Debug, Default, Deserialize)]
struct Usage {
    #[serde(default)]
    input_tokens: i64,
    #[serde(default)]
    output_tokens: i64,
    #[serde(default)]
    cache_read_input_tokens: i64,
    #[serde(default)]
    cache_creation_input_tokens: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuotaLimits {
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub rate_limit_type: String,
    /// Unix seconds.
    #[serde(default)]
    pub resets_at: i64,
}

/// What one transcript line contributes to the index.
#[derive(Debug, PartialEq)]
pub enum Record {
    Assistant {
        message_id: String,
        session_id: String,
        ts: DateTime<Utc>,
        model: String,
        input: i64,
        output: i64,
        cache_read: i64,
        cache_creation: i64,
        cwd: Option<String>,
        quota: Option<QuotaLimits>,
    },
    Prompt {
        uuid: String,
        session_id: String,
        ts: DateTime<Utc>,
        cwd: Option<String>,
    },
}

/// Parse one JSONL line. Lines that can't matter are rejected by a substring check before
/// any JSON parsing, which keeps the first scan of hundreds of MB fast.
pub fn parse_line(line: &str) -> Option<Record> {
    let assistant = line.contains("\"type\":\"assistant\"");
    let user = !assistant && line.contains("\"type\":\"user\"");
    if !assistant && !user {
        return None;
    }
    let parsed: Line = serde_json::from_str(line).ok()?;
    let session_id = parsed.session_id?;
    let ts = parsed.timestamp?;
    let message = parsed.message?;
    match parsed.kind {
        "assistant" => {
            let model = message.model.filter(|m| !m.is_empty() && m != "<synthetic>")?;
            let usage = message.usage.unwrap_or_default();
            Some(Record::Assistant {
                message_id: message.id?,
                session_id,
                ts,
                model,
                input: usage.input_tokens.max(0),
                output: usage.output_tokens.max(0),
                cache_read: usage.cache_read_input_tokens.max(0),
                cache_creation: usage.cache_creation_input_tokens.max(0),
                cwd: parsed.cwd,
                quota: parsed.quota_limits,
            })
        }
        "user" => {
            if parsed.is_meta == Some(true) || message.role != Some("user") {
                return None;
            }
            // A typed prompt is a JSON string, or an array of text/image blocks. Tool results
            // (the harness talking back to the model) aren't prompts.
            let raw = message.content?.get();
            if raw.contains("\"tool_result\"") {
                return None;
            }
            Some(Record::Prompt {
                uuid: parsed.uuid?,
                session_id,
                ts,
                cwd: parsed.cwd,
            })
        }
        _ => None,
    }
}

fn local_parts(ts: DateTime<Utc>) -> (String, u32) {
    let local = ts.with_timezone(&Local);
    (local.format("%Y-%m-%d").to_string(), local.hour())
}

fn upsert_session(conn: &Connection, session_id: &str, cwd: Option<&str>, ts: i64) -> AppResult<()> {
    conn.execute(
        "INSERT INTO claude_sessions (session_id, cwd, first_ts, last_ts) VALUES (?1, ?2, ?3, ?3)
         ON CONFLICT(session_id) DO UPDATE SET
            first_ts = MIN(first_ts, excluded.first_ts),
            last_ts = MAX(last_ts, excluded.last_ts),
            cwd = CASE WHEN claude_sessions.cwd = '' THEN excluded.cwd ELSE claude_sessions.cwd END",
        params![session_id, cwd.unwrap_or(""), ts],
    )?;
    Ok(())
}

/// Store one record. Duplicate message ids / prompt uuids are ignored.
pub fn store(conn: &Connection, record: &Record) -> AppResult<()> {
    match record {
        Record::Assistant {
            message_id,
            session_id,
            ts,
            model,
            input,
            output,
            cache_read,
            cache_creation,
            cwd,
            ..
        } => {
            let (day, hour) = local_parts(*ts);
            conn.execute(
                "INSERT OR IGNORE INTO claude_messages
                 (message_id, session_id, ts, day, hour, model, input_tokens, output_tokens, cache_read, cache_creation)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                params![message_id, session_id, ts.timestamp(), day, hour, model, input, output, cache_read, cache_creation],
            )?;
            upsert_session(conn, session_id, cwd.as_deref(), ts.timestamp())
        }
        Record::Prompt {
            uuid,
            session_id,
            ts,
            cwd,
        } => {
            let (day, hour) = local_parts(*ts);
            conn.execute(
                "INSERT OR IGNORE INTO claude_prompts (uuid, session_id, ts, day, hour) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![uuid, session_id, ts.timestamp(), day, hour],
            )?;
            upsert_session(conn, session_id, cwd.as_deref(), ts.timestamp())
        }
    }
}

/// All transcript files under `root`.
pub fn transcript_files(root: &Path) -> Vec<PathBuf> {
    fn walk(dir: &Path, out: &mut Vec<PathBuf>, depth: usize) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() && depth < 4 {
                walk(&path, out, depth + 1);
            } else if path.extension().is_some_and(|e| e == "jsonl") {
                out.push(path);
            }
        }
    }
    let mut out = Vec::new();
    walk(root, &mut out, 0);
    out
}

/// Outcome of an indexing pass.
#[derive(Debug, Default, PartialEq)]
pub struct IndexStats {
    pub files_read: usize,
    pub records: usize,
    /// The newest rate-limit event seen in this pass, if any.
    pub latest_quota: Option<(DateTime<Utc>, QuotaLimits)>,
}

/// Read only the bytes appended since the last pass (from the stored offset), stopping at the
/// last complete line so a line being written right now is picked up next time.
pub fn index_file(conn: &mut Connection, path: &Path, stats: &mut IndexStats) -> AppResult<()> {
    let key = path.display().to_string();
    let size = std::fs::metadata(path)?.len();
    let stored: Option<(i64, i64)> = conn
        .query_row(
            "SELECT offset, size FROM claude_files WHERE path = ?1",
            [&key],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .ok();
    let mut offset = stored.map(|(o, _)| o as u64).unwrap_or(0);
    if offset > size {
        offset = 0; // truncated or replaced: re-read (duplicates are ignored)
    }
    if offset == size {
        return Ok(());
    }

    let mut reader = BufReader::with_capacity(256 * 1024, File::open(path)?);
    reader.seek(SeekFrom::Start(offset))?;
    let tx = conn.transaction()?;
    let mut line = String::new();
    loop {
        line.clear();
        let read = reader.read_line(&mut line)?;
        if read == 0 || !line.ends_with('\n') {
            break; // EOF, or a partial line still being written
        }
        offset += read as u64;
        if let Some(record) = parse_line(line.trim_end()) {
            if let Record::Assistant {
                ts, quota: Some(q), ..
            } = &record
            {
                if stats.latest_quota.as_ref().is_none_or(|(t, _)| ts > t) {
                    stats.latest_quota = Some((*ts, q.clone()));
                }
            }
            store(&tx, &record)?;
            stats.records += 1;
        }
    }
    tx.execute(
        "INSERT INTO claude_files (path, offset, size) VALUES (?1, ?2, ?3)
         ON CONFLICT(path) DO UPDATE SET offset = excluded.offset, size = excluded.size",
        params![key, offset as i64, size as i64],
    )?;
    tx.commit()?;
    stats.files_read += 1;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const ASSISTANT: &str = r#"{"type":"assistant","sessionId":"s1","timestamp":"2026-09-28T10:00:00.000Z","cwd":"C:\\dev\\notch","message":{"id":"msg_1","model":"claude-opus-5-5","role":"assistant","content":[{"type":"text","text":"secret answer"}],"usage":{"input_tokens":10,"output_tokens":20,"cache_read_input_tokens":300,"cache_creation_input_tokens":4}}}"#;
    const PROMPT: &str = r#"{"type":"user","sessionId":"s1","uuid":"u1","timestamp":"2026-09-28T09:59:00.000Z","cwd":"C:\\dev\\notch","message":{"role":"user","content":"secret prompt"}}"#;
    const TOOL_RESULT: &str = r#"{"type":"user","sessionId":"s1","uuid":"u2","timestamp":"2026-09-28T10:00:01.000Z","message":{"role":"user","content":[{"type":"tool_result","tool_use_id":"t","content":"ok"}]}}"#;
    const META: &str = r#"{"type":"user","isMeta":true,"sessionId":"s1","uuid":"u3","timestamp":"2026-09-28T10:00:02.000Z","message":{"role":"user","content":"<command>"}}"#;
    const SYNTHETIC: &str = r#"{"type":"assistant","sessionId":"s1","timestamp":"2026-09-28T10:00:03.000Z","message":{"id":"msg_2","model":"<synthetic>","usage":{"input_tokens":0,"output_tokens":0}}}"#;
    const LIMIT: &str = r#"{"type":"assistant","sessionId":"s1","timestamp":"2026-09-28T11:00:00.000Z","quotaLimits":{"status":"rejected","rateLimitType":"five_hour","resetsAt":1790108400},"message":{"id":"msg_3","model":"claude-opus-5-5","usage":{"input_tokens":1,"output_tokens":1}}}"#;

    #[test]
    fn extracts_usage_and_prompts_but_skips_tool_results_and_meta() {
        match parse_line(ASSISTANT).unwrap() {
            Record::Assistant {
                message_id,
                model,
                input,
                output,
                cache_read,
                cache_creation,
                ..
            } => {
                assert_eq!(
                    (message_id.as_str(), model.as_str()),
                    ("msg_1", "claude-opus-5-5")
                );
                assert_eq!((input, output, cache_read, cache_creation), (10, 20, 300, 4));
            }
            other => panic!("unexpected {other:?}"),
        }
        assert!(matches!(parse_line(PROMPT), Some(Record::Prompt { .. })));
        assert_eq!(parse_line(TOOL_RESULT), None);
        assert_eq!(parse_line(META), None);
        assert_eq!(parse_line(SYNTHETIC), None);
        assert_eq!(parse_line(r#"{"type":"attachment","sessionId":"s1"}"#), None);
        assert_eq!(parse_line("not json but mentions \"type\":\"user\""), None);
    }

    #[test]
    fn reads_rate_limit_events() {
        match parse_line(LIMIT).unwrap() {
            Record::Assistant { quota: Some(q), .. } => {
                assert_eq!(q.rate_limit_type, "five_hour");
                assert_eq!(q.resets_at, 1_790_108_400);
            }
            other => panic!("unexpected {other:?}"),
        }
    }

    fn db() -> Connection {
        let mut conn = Connection::open_in_memory().unwrap();
        crate::db::migrate_for_tests(&mut conn);
        conn
    }

    #[test]
    fn indexes_incrementally_dedupes_and_never_stores_text() {
        let dir = std::env::temp_dir().join(format!("notch-claude-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("session.jsonl");
        // The same assistant message appears twice (once per content block), plus a partial line.
        std::fs::write(
            &file,
            format!("{PROMPT}\n{ASSISTANT}\n{ASSISTANT}\n{TOOL_RESULT}\n{{\"type\":\"user\""),
        )
        .unwrap();

        let mut conn = db();
        let mut stats = IndexStats::default();
        index_file(&mut conn, &file, &mut stats).unwrap();
        fn count(conn: &Connection, sql: &str) -> i64 {
            conn.query_row(sql, [], |r| r.get::<_, i64>(0)).unwrap()
        }
        assert_eq!(count(&conn, "SELECT COUNT(*) FROM claude_messages"), 1);
        assert_eq!(count(&conn, "SELECT SUM(output_tokens) FROM claude_messages"), 20);
        assert_eq!(count(&conn, "SELECT COUNT(*) FROM claude_prompts"), 1);
        assert_eq!(count(&conn, "SELECT COUNT(*) FROM claude_sessions"), 1);

        // Append: the partial line completes and a rate-limit event arrives.
        let mut text = std::fs::read_to_string(&file).unwrap();
        text.push_str(&format!(
            "{}\n{LIMIT}\n",
            &PROMPT.replace("\"u1\"", "\"u9\"")[14..]
        ));
        std::fs::write(&file, text).unwrap();
        let mut stats = IndexStats::default();
        index_file(&mut conn, &file, &mut stats).unwrap();
        assert_eq!(count(&conn, "SELECT COUNT(*) FROM claude_messages"), 2);
        assert!(stats.latest_quota.is_some());

        // Nothing new: nothing read.
        let mut stats = IndexStats::default();
        index_file(&mut conn, &file, &mut stats).unwrap();
        assert_eq!(stats.files_read, 0);

        // No text from the transcript made it into the database.
        let dump: String = conn
            .query_row(
                "SELECT group_concat(model || session_id || message_id) FROM claude_messages",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert!(!dump.contains("secret"));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
