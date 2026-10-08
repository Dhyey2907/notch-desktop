//! Focus sessions: a countdown (25/50/90 min) during which the notch holds back non-urgent
//! cards. One session at a time, shared by every screen; it survives a restart. Finished
//! sessions are counted per local day (for the pet's streak).

pub mod service;

use chrono::{DateTime, Local, TimeZone, Utc};
use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;
use ts_rs::TS;

use crate::error::{AppError, AppResult};

pub const MAX_MINUTES: u32 = 180;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct FocusState {
    /// Set while a session runs.
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ends_at: Option<DateTime<Utc>>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub started_at: Option<DateTime<Utc>>,
    /// Sessions finished today (local day).
    pub done_today: u32,
}

fn ts(secs: i64) -> DateTime<Utc> {
    Utc.timestamp_opt(secs, 0).single().unwrap_or_default()
}

fn day(at: DateTime<Utc>) -> String {
    at.with_timezone(&Local).format("%Y-%m-%d").to_string()
}

pub fn start(conn: &Connection, minutes: u32, now: DateTime<Utc>) -> AppResult<()> {
    if !(1..=MAX_MINUTES).contains(&minutes) {
        return Err(AppError::InvalidInput("Focus for 1 to 180 minutes".into()));
    }
    conn.execute(
        "INSERT INTO focus_current (id, started_at, ends_at) VALUES (1, ?1, ?2)
         ON CONFLICT(id) DO UPDATE SET started_at = ?1, ends_at = ?2",
        params![now.timestamp(), now.timestamp() + i64::from(minutes) * 60],
    )?;
    Ok(())
}

pub fn stop(conn: &Connection) -> AppResult<()> {
    conn.execute("DELETE FROM focus_current", [])?;
    Ok(())
}

pub fn current(conn: &Connection) -> AppResult<Option<(DateTime<Utc>, DateTime<Utc>)>> {
    Ok(conn
        .query_row(
            "SELECT started_at, ends_at FROM focus_current WHERE id = 1",
            [],
            |r| Ok((ts(r.get(0)?), ts(r.get(1)?))),
        )
        .optional()?)
}

/// Finish the session if its time is up: count it and clear it. Returns true if it finished.
pub fn finish_if_due(conn: &Connection, now: DateTime<Utc>) -> AppResult<bool> {
    let Some((_, ends_at)) = current(conn)? else {
        return Ok(false);
    };
    if ends_at > now {
        return Ok(false);
    }
    conn.execute(
        "INSERT INTO focus_days (day, sessions) VALUES (?1, 1)
         ON CONFLICT(day) DO UPDATE SET sessions = sessions + 1",
        [day(ends_at)],
    )?;
    stop(conn)?;
    Ok(true)
}

pub fn state(conn: &Connection, now: DateTime<Utc>) -> AppResult<FocusState> {
    let current = current(conn)?;
    let done_today: u32 = conn
        .query_row(
            "SELECT sessions FROM focus_days WHERE day = ?1",
            [day(now)],
            |r| r.get(0),
        )
        .optional()?
        .unwrap_or(0);
    Ok(FocusState {
        ends_at: current.map(|(_, e)| e),
        started_at: current.map(|(s, _)| s),
        done_today,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn db() -> Connection {
        let mut conn = Connection::open_in_memory().unwrap();
        crate::db::migrate_for_tests(&mut conn);
        conn
    }

    fn at(secs: i64) -> DateTime<Utc> {
        Utc.timestamp_opt(1_790_000_000 + secs, 0).unwrap()
    }

    #[test]
    fn runs_and_counts_sessions() {
        let conn = db();
        start(&conn, 25, at(0)).unwrap();
        let s = state(&conn, at(10)).unwrap();
        assert_eq!(s.ends_at, Some(at(25 * 60)));
        assert!(!finish_if_due(&conn, at(24 * 60)).unwrap(), "not yet");
        assert!(finish_if_due(&conn, at(25 * 60)).unwrap());
        let s = state(&conn, at(25 * 60 + 1)).unwrap();
        assert_eq!((s.ends_at, s.done_today), (None, 1));
        start(&conn, 50, at(30 * 60)).unwrap();
        assert!(
            finish_if_due(&conn, at(90 * 60)).unwrap(),
            "finished while Overline was closed"
        );
        assert_eq!(state(&conn, at(91 * 60)).unwrap().done_today, 2);
    }

    #[test]
    fn stopping_early_does_not_count() {
        let conn = db();
        start(&conn, 25, at(0)).unwrap();
        stop(&conn).unwrap();
        assert!(!finish_if_due(&conn, at(60 * 60)).unwrap());
        assert_eq!(state(&conn, at(60 * 60)).unwrap().done_today, 0);
        assert!(start(&conn, 0, at(0)).is_err());
        assert!(start(&conn, MAX_MINUTES + 1, at(0)).is_err());
    }
}
