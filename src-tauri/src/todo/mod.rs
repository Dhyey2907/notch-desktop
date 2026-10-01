//! To-do module: tasks with optional timers, and notes. Stored locally in SQLite.
//!
//! When a task's timer runs out, `service` announces it (and optionally rings) until you
//! acknowledge it — Done, +5 min, Dismiss — from any screen's notch.

pub mod service;

use chrono::{DateTime, TimeZone, Utc};
use rusqlite::{params, Connection, OptionalExtension, Row};
use serde::Serialize;
use ts_rs::TS;

use crate::error::{AppError, AppResult};

const MAX_TITLE: usize = 200;
const MAX_NOTE: usize = 10_000;
const MAX_TASKS: i64 = 500;
const MAX_NOTES: i64 = 200;
/// Longest timer: one day.
pub const MAX_TIMER_MINUTES: u32 = 24 * 60;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Task {
    pub id: String,
    pub title: String,
    pub done: bool,
    pub created_at: DateTime<Utc>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub done_at: Option<DateTime<Utc>>,
    /// Set while a timer is running.
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timer_ends_at: Option<DateTime<Utc>>,
    /// Length of the most recent timer, in seconds.
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timer_secs: Option<u32>,
    /// Ring when the timer finishes.
    pub ring: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Note {
    pub id: String,
    pub body: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct TodoSnapshot {
    /// Open tasks first (newest first), then done ones (most recently done first).
    pub tasks: Vec<Task>,
    /// Most recently edited first.
    pub notes: Vec<Note>,
    /// Tasks whose timer finished and hasn't been acknowledged yet.
    pub alerting: Vec<String>,
}

fn ts(secs: i64) -> DateTime<Utc> {
    Utc.timestamp_opt(secs, 0).single().unwrap_or_default()
}

fn task_from(row: &Row<'_>) -> rusqlite::Result<Task> {
    Ok(Task {
        id: row.get(0)?,
        title: row.get(1)?,
        done: row.get(2)?,
        created_at: ts(row.get(3)?),
        done_at: row.get::<_, Option<i64>>(4)?.map(ts),
        timer_ends_at: row.get::<_, Option<i64>>(5)?.map(ts),
        timer_secs: row.get(6)?,
        ring: row.get(7)?,
    })
}

const TASK_COLUMNS: &str = "id, title, done, created_at, done_at, timer_ends_at, timer_secs, ring";

fn clean_title(title: &str) -> AppResult<String> {
    let title = title.trim();
    if title.is_empty() {
        return Err(AppError::InvalidInput("Give the task a name".into()));
    }
    Ok(title.chars().take(MAX_TITLE).collect())
}

fn check_minutes(minutes: u32) -> AppResult<()> {
    if (1..=MAX_TIMER_MINUTES).contains(&minutes) {
        Ok(())
    } else {
        Err(AppError::InvalidInput(
            "Timers can be 1 minute to 24 hours".into(),
        ))
    }
}

fn changed(rows: usize) -> AppResult<()> {
    if rows == 0 {
        Err(AppError::InvalidInput("That item no longer exists".into()))
    } else {
        Ok(())
    }
}

pub fn get_task(conn: &Connection, id: &str) -> AppResult<Option<Task>> {
    Ok(conn
        .query_row(
            &format!("SELECT {TASK_COLUMNS} FROM todo_tasks WHERE id = ?1"),
            [id],
            task_from,
        )
        .optional()?)
}

pub fn list_tasks(conn: &Connection) -> AppResult<Vec<Task>> {
    let mut stmt = conn.prepare(&format!(
        "SELECT {TASK_COLUMNS} FROM todo_tasks
         ORDER BY done, CASE WHEN done THEN done_at ELSE created_at END DESC, rowid DESC"
    ))?;
    let tasks = stmt.query_map([], task_from)?.collect::<Result<_, _>>()?;
    Ok(tasks)
}

pub fn add_task(
    conn: &Connection,
    title: &str,
    timer_minutes: Option<u32>,
    ring: bool,
    now: DateTime<Utc>,
) -> AppResult<Task> {
    let title = clean_title(title)?;
    if let Some(minutes) = timer_minutes {
        check_minutes(minutes)?;
    }
    let count: i64 = conn.query_row("SELECT COUNT(*) FROM todo_tasks", [], |r| r.get(0))?;
    if count >= MAX_TASKS {
        return Err(AppError::InvalidInput(
            "That's a lot of tasks — clear some done ones first".into(),
        ));
    }
    let id = uuid::Uuid::new_v4().to_string();
    let secs = timer_minutes.map(|m| m * 60);
    conn.execute(
        "INSERT INTO todo_tasks (id, title, created_at, timer_ends_at, timer_secs, ring)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            id,
            title,
            now.timestamp(),
            secs.map(|s| now.timestamp() + i64::from(s)),
            secs,
            ring
        ],
    )?;
    get_task(conn, &id)?.ok_or_else(|| AppError::InvalidInput("task vanished".into()))
}

/// A task that rings at `at` (e.g. "Remind me" on a copied date). No length is recorded, so
/// the alert reads as a reminder rather than a timer.
pub fn add_reminder(
    conn: &Connection,
    title: &str,
    at: DateTime<Utc>,
    now: DateTime<Utc>,
) -> AppResult<Task> {
    let title = clean_title(title)?;
    if at <= now || at > now + chrono::Duration::days(366) {
        return Err(AppError::InvalidInput("Pick a time in the next year".into()));
    }
    let count: i64 = conn.query_row("SELECT COUNT(*) FROM todo_tasks", [], |r| r.get(0))?;
    if count >= MAX_TASKS {
        return Err(AppError::InvalidInput(
            "That's a lot of tasks — clear some done ones first".into(),
        ));
    }
    let id = uuid::Uuid::new_v4().to_string();
    conn.execute(
        "INSERT INTO todo_tasks (id, title, created_at, timer_ends_at, timer_secs, ring)
         VALUES (?1, ?2, ?3, ?4, NULL, 1)",
        params![id, title, now.timestamp(), at.timestamp()],
    )?;
    get_task(conn, &id)?.ok_or_else(|| AppError::InvalidInput("task vanished".into()))
}

pub fn set_done(conn: &Connection, id: &str, done: bool, now: DateTime<Utc>) -> AppResult<()> {
    // Finishing a task also stops its timer.
    changed(conn.execute(
        "UPDATE todo_tasks SET done = ?2, done_at = ?3,
             timer_ends_at = CASE WHEN ?2 THEN NULL ELSE timer_ends_at END
         WHERE id = ?1",
        params![id, done, done.then(|| now.timestamp())],
    )?)
}

pub fn rename(conn: &Connection, id: &str, title: &str) -> AppResult<()> {
    let title = clean_title(title)?;
    changed(conn.execute(
        "UPDATE todo_tasks SET title = ?2 WHERE id = ?1",
        params![id, title],
    )?)
}

pub fn delete_task(conn: &Connection, id: &str) -> AppResult<()> {
    changed(conn.execute("DELETE FROM todo_tasks WHERE id = ?1", [id])?)
}

/// Remove every finished task.
pub fn clear_done(conn: &Connection) -> AppResult<usize> {
    Ok(conn.execute("DELETE FROM todo_tasks WHERE done", [])?)
}

pub fn start_timer(
    conn: &Connection,
    id: &str,
    minutes: u32,
    ring: bool,
    now: DateTime<Utc>,
) -> AppResult<()> {
    check_minutes(minutes)?;
    let secs = minutes * 60;
    changed(conn.execute(
        "UPDATE todo_tasks SET timer_ends_at = ?2, timer_secs = ?3, ring = ?4, done = 0, done_at = NULL
         WHERE id = ?1",
        params![id, now.timestamp() + i64::from(secs), secs, ring],
    )?)
}

pub fn cancel_timer(conn: &Connection, id: &str) -> AppResult<()> {
    changed(conn.execute("UPDATE todo_tasks SET timer_ends_at = NULL WHERE id = ?1", [id])?)
}

/// When the next running timer ends.
pub fn next_timer_at(conn: &Connection) -> AppResult<Option<DateTime<Utc>>> {
    let at: Option<i64> = conn.query_row(
        "SELECT MIN(timer_ends_at) FROM todo_tasks WHERE timer_ends_at IS NOT NULL",
        [],
        |r| r.get(0),
    )?;
    Ok(at.map(ts))
}

/// Take every timer that has run out: clears them and returns their tasks (with the time
/// each was due in `timer_ends_at`, so late ones can be told apart).
pub fn take_due_timers(conn: &mut Connection, now: DateTime<Utc>) -> AppResult<Vec<Task>> {
    let tx = conn.transaction()?;
    let due: Vec<Task> = {
        let mut stmt = tx.prepare(&format!(
            "SELECT {TASK_COLUMNS} FROM todo_tasks
             WHERE timer_ends_at IS NOT NULL AND timer_ends_at <= ?1 ORDER BY timer_ends_at"
        ))?;
        let rows = stmt.query_map([now.timestamp()], task_from)?;
        rows.collect::<Result<_, _>>()?
    };
    tx.execute(
        "UPDATE todo_tasks SET timer_ends_at = NULL
         WHERE timer_ends_at IS NOT NULL AND timer_ends_at <= ?1",
        [now.timestamp()],
    )?;
    tx.commit()?;
    Ok(due)
}

pub fn list_notes(conn: &Connection) -> AppResult<Vec<Note>> {
    let mut stmt = conn.prepare(
        "SELECT id, body, created_at, updated_at FROM todo_notes ORDER BY updated_at DESC, rowid DESC",
    )?;
    let notes = stmt
        .query_map([], |row| {
            Ok(Note {
                id: row.get(0)?,
                body: row.get(1)?,
                created_at: ts(row.get(2)?),
                updated_at: ts(row.get(3)?),
            })
        })?
        .collect::<Result<_, _>>()?;
    Ok(notes)
}

fn check_note(body: &str) -> AppResult<()> {
    if body.chars().count() > MAX_NOTE {
        Err(AppError::InvalidInput(
            "Notes can be up to 10,000 characters".into(),
        ))
    } else {
        Ok(())
    }
}

pub fn add_note(conn: &Connection, body: &str, now: DateTime<Utc>) -> AppResult<Note> {
    check_note(body)?;
    let count: i64 = conn.query_row("SELECT COUNT(*) FROM todo_notes", [], |r| r.get(0))?;
    if count >= MAX_NOTES {
        return Err(AppError::InvalidInput(
            "That's a lot of notes — delete some first".into(),
        ));
    }
    let id = uuid::Uuid::new_v4().to_string();
    conn.execute(
        "INSERT INTO todo_notes (id, body, created_at, updated_at) VALUES (?1, ?2, ?3, ?3)",
        params![id, body, now.timestamp()],
    )?;
    Ok(Note {
        id,
        body: body.to_owned(),
        created_at: ts(now.timestamp()),
        updated_at: ts(now.timestamp()),
    })
}

pub fn update_note(conn: &Connection, id: &str, body: &str, now: DateTime<Utc>) -> AppResult<()> {
    check_note(body)?;
    changed(conn.execute(
        "UPDATE todo_notes SET body = ?2, updated_at = ?3 WHERE id = ?1",
        params![id, body, now.timestamp()],
    )?)
}

pub fn delete_note(conn: &Connection, id: &str) -> AppResult<()> {
    changed(conn.execute("DELETE FROM todo_notes WHERE id = ?1", [id])?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    fn db() -> Connection {
        let mut conn = Connection::open_in_memory().unwrap();
        crate::db::migrate_for_tests(&mut conn);
        conn
    }

    fn at(secs: i64) -> DateTime<Utc> {
        ts(1_800_000_000 + secs)
    }

    #[test]
    fn adds_tasks_with_and_without_timers() {
        let conn = db();
        let plain = add_task(&conn, "  Buy milk  ", None, false, at(0)).unwrap();
        assert_eq!(plain.title, "Buy milk");
        assert!(plain.timer_ends_at.is_none() && !plain.done);
        let timed = add_task(&conn, "Write report", Some(25), true, at(10)).unwrap();
        assert_eq!(timed.timer_ends_at, Some(at(10 + 25 * 60)));
        assert_eq!((timed.timer_secs, timed.ring), (Some(1500), true));
        assert!(add_task(&conn, "   ", None, false, at(0)).is_err());
        assert!(add_task(&conn, "x", Some(0), false, at(0)).is_err());
        assert!(add_task(&conn, "x", Some(MAX_TIMER_MINUTES + 1), false, at(0)).is_err());
        // Newest open task first.
        let titles: Vec<_> = list_tasks(&conn).unwrap().into_iter().map(|t| t.title).collect();
        assert_eq!(titles, ["Write report", "Buy milk"]);
    }

    #[test]
    fn done_tasks_sink_and_lose_their_timer() {
        let conn = db();
        let a = add_task(&conn, "a", Some(5), false, at(0)).unwrap();
        add_task(&conn, "b", None, false, at(1)).unwrap();
        set_done(&conn, &a.id, true, at(2)).unwrap();
        let tasks = list_tasks(&conn).unwrap();
        assert_eq!(tasks[0].title, "b");
        assert!(tasks[1].done && tasks[1].timer_ends_at.is_none() && tasks[1].done_at == Some(at(2)));
        set_done(&conn, &a.id, false, at(3)).unwrap();
        assert!(!get_task(&conn, &a.id).unwrap().unwrap().done);
        assert_eq!(clear_done(&conn).unwrap(), 0);
        set_done(&conn, &a.id, true, at(4)).unwrap();
        assert_eq!(clear_done(&conn).unwrap(), 1);
        assert!(set_done(&conn, "missing", true, at(0)).is_err());
    }

    #[test]
    fn timers_fire_in_order_once_and_can_be_snoozed() {
        let mut conn = db();
        let late = add_task(&conn, "late", Some(1), true, at(0)).unwrap();
        let later = add_task(&conn, "later", Some(3), false, at(0)).unwrap();
        assert_eq!(next_timer_at(&conn).unwrap(), Some(at(60)));
        assert!(take_due_timers(&mut conn, at(59)).unwrap().is_empty());

        let due = take_due_timers(&mut conn, at(61)).unwrap();
        assert_eq!(due.len(), 1);
        assert_eq!(
            (due[0].id.as_str(), due[0].timer_ends_at),
            (late.id.as_str(), Some(at(60)))
        );
        assert!(take_due_timers(&mut conn, at(62)).unwrap().is_empty()); // fires once
        assert_eq!(next_timer_at(&conn).unwrap(), Some(at(180)));

        // Snooze = start again for a few minutes.
        start_timer(&conn, &late.id, 5, true, at(70)).unwrap();
        assert_eq!(next_timer_at(&conn).unwrap(), Some(at(180)));
        cancel_timer(&conn, &later.id).unwrap();
        assert_eq!(next_timer_at(&conn).unwrap(), Some(at(70 + 300)));

        // Timers that ran out while the app was closed all fire at once on start.
        let due = take_due_timers(&mut conn, at(0) + Duration::days(1)).unwrap();
        assert_eq!(due.len(), 1);
        assert_eq!(next_timer_at(&conn).unwrap(), None);
    }

    #[test]
    fn edits_and_deletes() {
        let conn = db();
        let t = add_task(&conn, "old", None, false, at(0)).unwrap();
        rename(&conn, &t.id, "new").unwrap();
        assert_eq!(get_task(&conn, &t.id).unwrap().unwrap().title, "new");
        assert!(rename(&conn, &t.id, "").is_err());
        delete_task(&conn, &t.id).unwrap();
        assert!(get_task(&conn, &t.id).unwrap().is_none());
        assert!(delete_task(&conn, &t.id).is_err());
    }

    #[test]
    fn notes_keep_most_recently_edited_first() {
        let conn = db();
        let a = add_note(&conn, "first", at(0)).unwrap();
        let b = add_note(&conn, "second", at(1)).unwrap();
        assert_eq!(list_notes(&conn).unwrap()[0].id, b.id);
        update_note(&conn, &a.id, "first, edited", at(2)).unwrap();
        let notes = list_notes(&conn).unwrap();
        assert_eq!(
            (notes[0].id.as_str(), notes[0].body.as_str()),
            (a.id.as_str(), "first, edited")
        );
        assert!(update_note(&conn, &a.id, &"x".repeat(MAX_NOTE + 1), at(3)).is_err());
        delete_note(&conn, &b.id).unwrap();
        assert_eq!(list_notes(&conn).unwrap().len(), 1);
    }

    #[test]
    fn reminders_can_be_days_ahead() {
        let conn = db();
        let task = add_reminder(&conn, "Dentist", at(3 * 24 * 3600), at(0)).unwrap();
        assert_eq!(task.timer_ends_at, Some(at(3 * 24 * 3600)));
        assert_eq!(task.timer_secs, None, "a reminder, not a timer");
        assert!(task.ring);
        assert!(add_reminder(&conn, "Past", at(-60), at(0)).is_err());
        assert!(add_reminder(&conn, "Far", at(400 * 24 * 3600), at(0)).is_err());
    }
}
