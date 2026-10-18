//! Routines learned from habits: the first time each app comes to the front each day is noted
//! (program and minute, nothing about what's in it). When two or more apps keep starting
//! together at about the same time on most recent days, Overline offers a Desk routine that
//! opens them for you at that time.

use std::collections::HashMap;

use rusqlite::{params, Connection};
use serde::Serialize;
use ts_rs::TS;

use crate::error::AppResult;

/// Look back this many days that have any app starts.
pub const LOOKBACK_DAYS: usize = 5;
/// An app counts if it started on at least this many of them…
pub const MIN_DAYS: usize = 3;
/// …within this many minutes of its usual time.
pub const SPREAD_MIN: u32 = 30;
/// Apps whose usual times are this close make one routine.
pub const TOGETHER_MIN: u32 = 15;
/// Records older than this are dropped.
const KEEP_DAYS: i64 = 30;

/// Never suggested: Windows' own parts and Overline.
const SKIP: &[&str] = &[
    "explorer",
    "notch-desktop",
    "searchhost",
    "shellexperiencehost",
    "startmenuexperiencehost",
    "lockapp",
    "applicationframehost",
    "textinputhost",
    "systemsettings",
    "taskmgr",
    "mmc",
    "rundll32",
];

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct HabitSuggestion {
    /// File stems, e.g. "spotify".
    pub exes: Vec<String>,
    /// Full paths to start them.
    pub paths: Vec<String>,
    /// Minutes after midnight to run it (a little before the usual time).
    pub at_min: u32,
    /// A stable key, so a dismissed suggestion isn't offered again.
    pub key: String,
}

/// The shell saw `exe` come to the front: note it, and once a day look for a habit worth a
/// routine (unless that's off, or it's been offered and turned down).
pub fn seen(app: &tauri::AppHandle, exe: &str, path: &str) {
    use tauri::Manager;
    static CHECKED: std::sync::Mutex<Option<chrono::NaiveDate>> = std::sync::Mutex::new(None);
    let state = app.state::<crate::state::AppState>();
    let off = state
        .settings
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .desk
        .no_habit_routines;
    if off {
        return;
    }
    let now = chrono::Local::now();
    let today = now.date_naive();
    let day = today.format("%Y-%m-%d").to_string();
    let minute = chrono::Timelike::hour(&now) * 60 + chrono::Timelike::minute(&now);
    let _ = state.db.with(|c| note(c, &day, exe, path, minute));
    {
        let mut checked = CHECKED.lock().unwrap_or_else(|e| e.into_inner());
        if *checked == Some(today) {
            return;
        }
        *checked = Some(today);
    }
    let _ = state.db.with(|c| prune(c, today));
    let Ok(rows) = state.db.with(recent) else { return };
    let routines = state
        .settings
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .desk
        .routines
        .clone();
    for s in detect(&rows) {
        let dismissed = state.db.with(|c| is_dismissed(c, &s.key)).unwrap_or(false);
        // Already a routine that opens all of these.
        let covered = routines.iter().any(|r| {
            s.paths.iter().all(|p| {
                r.steps.iter().any(
                    |st| matches!(st, crate::desk::RoutineStep::App { path } if path.eq_ignore_ascii_case(p)),
                )
            })
        });
        if !dismissed && !covered {
            state.bus.emit("habits", crate::events::AppEvent::HabitFound(s));
            break;
        }
    }
}

/// Don't offer this one again.
pub fn dismiss(conn: &Connection, key: &str) -> AppResult<()> {
    conn.execute(
        "INSERT OR REPLACE INTO kv_cache (key, value, fetched_at) VALUES (?1, '1', datetime('now'))",
        params![format!("habit.dismissed.{key}")],
    )?;
    Ok(())
}

fn is_dismissed(conn: &Connection, key: &str) -> AppResult<bool> {
    let n: i64 = conn.query_row(
        "SELECT COUNT(*) FROM kv_cache WHERE key = ?1",
        params![format!("habit.dismissed.{key}")],
        |r| r.get(0),
    )?;
    Ok(n > 0)
}

/// Note an app coming to the front (only its first time each day counts).
pub fn note(conn: &Connection, day: &str, exe: &str, path: &str, minute: u32) -> AppResult<()> {
    if exe.is_empty() || SKIP.contains(&exe) {
        return Ok(());
    }
    conn.execute(
        "INSERT OR IGNORE INTO app_starts (day, exe, path, minute) VALUES (?1, ?2, ?3, ?4)",
        params![day, exe, path, minute],
    )?;
    Ok(())
}

pub fn prune(conn: &Connection, today: chrono::NaiveDate) -> AppResult<()> {
    let cutoff = (today - chrono::Duration::days(KEEP_DAYS))
        .format("%Y-%m-%d")
        .to_string();
    conn.execute("DELETE FROM app_starts WHERE day < ?1", params![cutoff])?;
    Ok(())
}

/// (day, exe, path, minute) rows from the last `LOOKBACK_DAYS` days that have any.
pub fn recent(conn: &Connection) -> AppResult<Vec<(String, String, String, u32)>> {
    let mut stmt = conn.prepare(
        "SELECT day, exe, path, minute FROM app_starts
         WHERE day IN (SELECT DISTINCT day FROM app_starts ORDER BY day DESC LIMIT ?1)",
    )?;
    let rows = stmt.query_map(params![LOOKBACK_DAYS as i64], |r| {
        Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get::<_, i64>(3)? as u32))
    })?;
    Ok(rows.filter_map(Result::ok).collect())
}

fn median(v: &mut [u32]) -> u32 {
    v.sort_unstable();
    v[v.len() / 2]
}

/// Groups of apps that start together at a steady time.
pub fn detect(rows: &[(String, String, String, u32)]) -> Vec<HabitSuggestion> {
    let days: std::collections::HashSet<&str> = rows.iter().map(|r| r.0.as_str()).collect();
    if days.len() < MIN_DAYS {
        return Vec::new();
    }
    // Per app: its first minute on each day, and a path.
    let mut by_app: HashMap<&str, (Vec<u32>, &str)> = HashMap::new();
    for (_, exe, path, minute) in rows {
        let entry = by_app.entry(exe).or_insert((Vec::new(), path));
        entry.0.push(*minute);
    }
    // Steady apps: on enough days, close to their usual time.
    let mut steady: Vec<(&str, &str, u32)> = by_app
        .into_iter()
        .filter_map(|(exe, (mut minutes, path))| {
            let m = median(&mut minutes);
            let near = minutes.iter().filter(|x| x.abs_diff(m) <= SPREAD_MIN).count();
            (near >= MIN_DAYS).then_some((exe, path, m))
        })
        .collect();
    steady.sort_by_key(|(exe, _, m)| (*m, exe.to_string()));
    // Chain apps whose usual times are close into groups.
    let mut groups: Vec<Vec<(&str, &str, u32)>> = Vec::new();
    for app in steady {
        match groups.last_mut() {
            Some(g) if app.2 - g.last().map_or(app.2, |l| l.2) <= TOGETHER_MIN => g.push(app),
            _ => groups.push(vec![app]),
        }
    }
    groups
        .into_iter()
        .filter(|g| g.len() >= 2)
        .map(|g| {
            let mut exes: Vec<String> = g.iter().map(|a| a.0.to_string()).collect();
            let paths = g.iter().map(|a| a.1.to_string()).collect();
            let at_min = g.iter().map(|a| a.2).min().unwrap_or(0).saturating_sub(2);
            let mut sorted = exes.clone();
            sorted.sort();
            let key = format!("{}@{}", sorted.join("+"), at_min / 30);
            exes.truncate(8);
            HabitSuggestion {
                exes,
                paths,
                at_min,
                key,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rows(spec: &[(&str, &str, u32)]) -> Vec<(String, String, String, u32)> {
        spec.iter()
            .map(|(d, e, m)| (d.to_string(), e.to_string(), format!("C:\\{e}.exe"), *m))
            .collect()
    }

    #[test]
    fn finds_apps_that_start_together() {
        let mut spec = Vec::new();
        for (i, d) in ["d1", "d2", "d3", "d4"].iter().enumerate() {
            spec.push((*d, "code", 9 * 60 + 30 + i as u32 * 3));
            spec.push((*d, "spotify", 9 * 60 + 35));
            spec.push((*d, "chrome", if i == 3 { 14 * 60 } else { 9 * 60 + 40 }));
            spec.push((*d, "excel", 15 * 60 + i as u32 * 50));
        }
        let found = detect(&rows(&spec));
        assert_eq!(found.len(), 1);
        let g = &found[0];
        assert_eq!(g.exes, vec!["spotify", "code", "chrome"]);
        assert_eq!(g.at_min, 9 * 60 + 35 - 2);
        assert!(g.paths[1].ends_with("code.exe"));
    }

    #[test]
    fn needs_enough_days_and_two_apps() {
        let few = rows(&[
            ("d1", "code", 600),
            ("d1", "spotify", 600),
            ("d2", "code", 600),
            ("d2", "spotify", 600),
        ]);
        assert!(detect(&few).is_empty());
        let one = rows(&[("d1", "code", 600), ("d2", "code", 600), ("d3", "code", 600)]);
        assert!(detect(&one).is_empty());
    }

    #[test]
    fn notes_only_the_first_start_and_skips_windows_parts() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(include_str!("../db/migrations/019_app_starts.sql"))
            .unwrap();
        note(&conn, "2026-10-18", "code", "C:\\code.exe", 570).unwrap();
        note(&conn, "2026-10-18", "code", "C:\\code.exe", 600).unwrap();
        note(&conn, "2026-10-18", "explorer", "C:\\explorer.exe", 571).unwrap();
        let r = recent(&conn).unwrap();
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].3, 570);
    }
}
