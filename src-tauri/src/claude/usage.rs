//! Usage summaries computed from the local transcript index. These are local estimates of
//! what this PC's Claude Code sessions consumed, not official account quotas.

use chrono::{DateTime, Duration, Local, Utc};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::error::AppResult;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum UsageRange {
    All,
    Days30,
    Days7,
}

impl UsageRange {
    fn days(self) -> Option<i64> {
        match self {
            UsageRange::All => None,
            UsageRange::Days30 => Some(30),
            UsageRange::Days7 => Some(7),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ModelUsage {
    pub model: String,
    pub messages: u32,
    #[ts(type = "number")]
    pub tokens: u64,
    #[ts(type = "number")]
    pub output_tokens: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct DayTokens {
    pub date: String,
    #[ts(type = "number")]
    pub tokens: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct UsageSummary {
    pub range: UsageRange,
    pub sessions: u32,
    /// Prompts you typed (tool results excluded).
    pub messages: u32,
    #[ts(type = "number")]
    pub total_tokens: u64,
    #[ts(type = "number")]
    pub input_tokens: u64,
    #[ts(type = "number")]
    pub output_tokens: u64,
    #[ts(type = "number")]
    pub cache_read_tokens: u64,
    #[ts(type = "number")]
    pub cache_creation_tokens: u64,
    pub active_days: u32,
    /// Local hour (0–23) with the most activity.
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub peak_hour: Option<u32>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub favorite_model: Option<String>,
    pub models: Vec<ModelUsage>,
    /// Tokens in the equal-length period before this one (7d / 30d only).
    #[ts(optional, type = "number")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub previous_period_tokens: Option<u64>,
    /// Tokens per local day over the last 26 weeks, oldest first.
    pub daily: Vec<DayTokens>,
    /// Folder name of the project with the most activity in the range.
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_project: Option<String>,
}

/// Heatmap span, in days (26 weeks).
pub const DAILY_DAYS: i64 = 182;
const TOKEN_SUM: &str = "COALESCE(SUM(input_tokens + output_tokens + cache_read + cache_creation), 0)";

fn folder_name(cwd: &str) -> String {
    cwd.trim_end_matches(['\\', '/'])
        .rsplit(['\\', '/'])
        .next()
        .unwrap_or(cwd)
        .to_owned()
}

pub fn summarize(conn: &Connection, range: UsageRange, now: DateTime<Utc>) -> AppResult<UsageSummary> {
    let since = range
        .days()
        .map(|d| (now - Duration::days(d)).timestamp())
        .unwrap_or(0);

    let (input, output, cache_read, cache_creation): (i64, i64, i64, i64) = conn.query_row(
        "SELECT COALESCE(SUM(input_tokens),0), COALESCE(SUM(output_tokens),0),
                COALESCE(SUM(cache_read),0), COALESCE(SUM(cache_creation),0)
         FROM claude_messages WHERE ts >= ?1",
        [since],
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
    )?;
    let messages: u32 = conn.query_row(
        "SELECT COUNT(*) FROM claude_prompts WHERE ts >= ?1",
        [since],
        |r| r.get(0),
    )?;
    let sessions: u32 = conn.query_row(
        "SELECT COUNT(DISTINCT session_id) FROM (
            SELECT session_id FROM claude_messages WHERE ts >= ?1
            UNION ALL SELECT session_id FROM claude_prompts WHERE ts >= ?1)",
        [since],
        |r| r.get(0),
    )?;
    let active_days: u32 = conn.query_row(
        "SELECT COUNT(DISTINCT day) FROM (
            SELECT day FROM claude_messages WHERE ts >= ?1
            UNION ALL SELECT day FROM claude_prompts WHERE ts >= ?1)",
        [since],
        |r| r.get(0),
    )?;
    let peak_hour: Option<u32> = conn
        .query_row(
            "SELECT hour FROM claude_messages WHERE ts >= ?1 GROUP BY hour ORDER BY COUNT(*) DESC, hour LIMIT 1",
            [since],
            |r| r.get(0),
        )
        .optional()?;

    let mut stmt = conn.prepare(&format!(
        "SELECT model, COUNT(*), {TOKEN_SUM}, COALESCE(SUM(output_tokens), 0)
         FROM claude_messages WHERE ts >= ?1 GROUP BY model ORDER BY 3 DESC"
    ))?;
    let models: Vec<ModelUsage> = stmt
        .query_map([since], |r| {
            Ok(ModelUsage {
                model: r.get(0)?,
                messages: r.get(1)?,
                tokens: r.get::<_, i64>(2)?.max(0) as u64,
                output_tokens: r.get::<_, i64>(3)?.max(0) as u64,
            })
        })?
        .collect::<Result<_, _>>()?;
    // "Favorite" = the model you used for the most messages.
    let favorite_model = models.iter().max_by_key(|m| m.messages).map(|m| m.model.clone());

    let previous_period_tokens = match range.days() {
        Some(d) => {
            let prev_since = (now - Duration::days(2 * d)).timestamp();
            let tokens: i64 = conn.query_row(
                &format!("SELECT {TOKEN_SUM} FROM claude_messages WHERE ts >= ?1 AND ts < ?2"),
                params![prev_since, since],
                |r| r.get(0),
            )?;
            Some(tokens.max(0) as u64)
        }
        None => None,
    };

    let today = now.with_timezone(&Local).date_naive();
    let first_day = today - Duration::days(DAILY_DAYS - 1);
    let mut stmt = conn.prepare(&format!(
        "SELECT day, {TOKEN_SUM} FROM claude_messages WHERE day >= ?1 GROUP BY day"
    ))?;
    let by_day: std::collections::HashMap<String, i64> = stmt
        .query_map([first_day.format("%Y-%m-%d").to_string()], |r| {
            Ok((r.get(0)?, r.get(1)?))
        })?
        .collect::<Result<_, _>>()?;
    let daily = (0..DAILY_DAYS)
        .map(|i| {
            let date = (first_day + Duration::days(i)).format("%Y-%m-%d").to_string();
            let tokens = by_day.get(&date).copied().unwrap_or(0).max(0) as u64;
            DayTokens { date, tokens }
        })
        .collect();

    let top_project: Option<String> = conn
        .query_row(
            "SELECT s.cwd FROM claude_messages m JOIN claude_sessions s ON s.session_id = m.session_id
             WHERE m.ts >= ?1 AND s.cwd != '' GROUP BY s.cwd ORDER BY COUNT(*) DESC LIMIT 1",
            [since],
            |r| r.get::<_, String>(0),
        )
        .optional()?
        .map(|cwd| folder_name(&cwd));

    let [input, output, cache_read, cache_creation] =
        [input, output, cache_read, cache_creation].map(|v| v.max(0) as u64);
    Ok(UsageSummary {
        range,
        sessions,
        messages,
        total_tokens: input + output + cache_read + cache_creation,
        input_tokens: input,
        output_tokens: output,
        cache_read_tokens: cache_read,
        cache_creation_tokens: cache_creation,
        active_days,
        peak_hour,
        favorite_model,
        models,
        previous_period_tokens,
        daily,
        top_project,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::claude::transcripts::{store, Record};

    fn msg(id: &str, session: &str, ts: DateTime<Utc>, model: &str, output: i64) -> Record {
        Record::Assistant {
            message_id: id.into(),
            session_id: session.into(),
            ts,
            model: model.into(),
            input: 10,
            output,
            cache_read: 100,
            cache_creation: 0,
            cwd: Some(format!("C:\\dev\\{session}")),
            quota: None,
        }
    }

    #[test]
    fn summarizes_ranges_models_and_comparison() {
        let mut conn = Connection::open_in_memory().unwrap();
        crate::db::migrate_for_tests(&mut conn);
        let now = Utc::now();
        let day = Duration::days(1);
        for record in [
            msg("a", "proj-a", now - Duration::hours(2), "claude-opus-5-5", 50),
            msg("b", "proj-a", now - Duration::hours(1), "claude-opus-5-5", 50),
            msg("c", "proj-b", now - day * 3, "claude-sonnet-5", 20),
            msg("d", "proj-b", now - day * 10, "claude-opus-5", 5), // previous 7-day period
            msg("e", "proj-c", now - day * 100, "claude-opus-5", 5),
            Record::Prompt {
                uuid: "p1".into(),
                session_id: "proj-a".into(),
                ts: now - Duration::hours(2),
                cwd: None,
            },
        ] {
            store(&conn, &record).unwrap();
        }

        let week = summarize(&conn, UsageRange::Days7, now).unwrap();
        assert_eq!(week.sessions, 2);
        assert_eq!(week.messages, 1);
        assert_eq!(week.output_tokens, 120);
        assert_eq!(week.total_tokens, 3 * 110 + 120);
        assert_eq!(week.favorite_model.as_deref(), Some("claude-opus-5-5"));
        assert_eq!(week.previous_period_tokens, Some(115));
        assert_eq!(week.top_project.as_deref(), Some("proj-a"));
        assert!(week.active_days >= 1);
        assert_eq!(week.models.len(), 2);

        let all = summarize(&conn, UsageRange::All, now).unwrap();
        assert_eq!(all.sessions, 3);
        assert_eq!(all.previous_period_tokens, None);
        assert_eq!(all.daily.len() as i64, DAILY_DAYS);
        assert_eq!(all.daily.iter().map(|d| d.tokens).sum::<u64>(), 5 * 110 + 130);
    }

    #[test]
    fn empty_index_is_all_zeros() {
        let mut conn = Connection::open_in_memory().unwrap();
        crate::db::migrate_for_tests(&mut conn);
        let s = summarize(&conn, UsageRange::Days30, Utc::now()).unwrap();
        assert_eq!((s.sessions, s.messages, s.total_tokens), (0, 0, 0));
        assert_eq!(s.peak_hour, None);
        assert_eq!(s.favorite_model, None);
        assert_eq!(s.previous_period_tokens, Some(0));
    }
}
