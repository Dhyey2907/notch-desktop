//! The Claude session dashboard: tokens and an estimated cost per session today (from the
//! indexed transcripts), and bringing a session's terminal to the front.

use std::collections::HashMap;

use rusqlite::Connection;
use serde::Serialize;
use ts_rs::TS;

use crate::error::{AppError, AppResult};

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SessionUsage {
    pub session_id: String,
    #[ts(type = "number")]
    pub input_tokens: u64,
    #[ts(type = "number")]
    pub output_tokens: u64,
    /// Cache reads and writes together.
    #[ts(type = "number")]
    pub cache_tokens: u64,
    /// At API list prices (a subscription plan doesn't bill it).
    pub cost_usd: f64,
}

/// US dollars per million tokens: input, output, cache read, cache write.
pub fn prices(model: &str) -> (f64, f64, f64, f64) {
    let m = model.to_lowercase();
    if m.contains("opus") {
        (15.0, 75.0, 1.5, 18.75)
    } else if m.contains("haiku") {
        (1.0, 5.0, 0.1, 1.25)
    } else {
        // Sonnet, and anything unknown.
        (3.0, 15.0, 0.3, 3.75)
    }
}

pub fn cost(model: &str, input: u64, output: u64, cache_read: u64, cache_write: u64) -> f64 {
    let (i, o, r, w) = prices(model);
    (input as f64 * i + output as f64 * o + cache_read as f64 * r + cache_write as f64 * w) / 1_000_000.0
}

/// Tokens and cost per session for one day (`YYYY-MM-DD`, as stored), costliest first.
pub fn usage_for_day(conn: &Connection, day: &str) -> AppResult<Vec<SessionUsage>> {
    let mut stmt = conn.prepare(
        "SELECT session_id, model, SUM(input_tokens), SUM(output_tokens), SUM(cache_read), SUM(cache_creation)
         FROM claude_messages WHERE day = ?1 GROUP BY session_id, model",
    )?;
    let rows = stmt.query_map([day], |r| {
        Ok((
            r.get::<_, String>(0)?,
            r.get::<_, String>(1)?,
            r.get::<_, i64>(2)?.max(0) as u64,
            r.get::<_, i64>(3)?.max(0) as u64,
            r.get::<_, i64>(4)?.max(0) as u64,
            r.get::<_, i64>(5)?.max(0) as u64,
        ))
    })?;
    let mut by: HashMap<String, SessionUsage> = HashMap::new();
    for row in rows {
        let (id, model, input, output, read, write) = row?;
        let e = by.entry(id.clone()).or_insert_with(|| SessionUsage {
            session_id: id,
            input_tokens: 0,
            output_tokens: 0,
            cache_tokens: 0,
            cost_usd: 0.0,
        });
        e.input_tokens += input;
        e.output_tokens += output;
        e.cache_tokens += read + write;
        e.cost_usd += cost(&model, input, output, read, write);
    }
    let mut out: Vec<SessionUsage> = by.into_values().collect();
    out.sort_by(|a, b| b.cost_usd.total_cmp(&a.cost_usd));
    Ok(out)
}

/// Bring forward the window the session runs in: its process's, or the nearest parent's that
/// has one (the terminal). Windows Terminal hosts every tab in one window, so that window comes
/// forward, not the exact tab.
pub fn focus_process(pid: u32) -> AppResult<()> {
    #[cfg(windows)]
    {
        let parents = super::relay::win::process_table();
        let mut current = pid;
        for _ in 0..10 {
            if let Some(hwnd) = win::window_of(current) {
                return crate::desk::win::focus(hwnd);
            }
            match parents.get(&current) {
                Some((parent, _)) if *parent != 0 && *parent != current => current = *parent,
                _ => break,
            }
        }
        Err(AppError::InvalidInput(
            "Couldn't find that session's window".into(),
        ))
    }
    #[cfg(not(windows))]
    {
        let _ = pid;
        Err(AppError::Platform("not supported".into()))
    }
}

#[cfg(windows)]
mod win {
    use windows::core::BOOL;
    use windows::Win32::{
        Foundation::{HWND, LPARAM},
        UI::WindowsAndMessaging::{
            EnumWindows, GetWindow, GetWindowTextLengthW, GetWindowThreadProcessId, IsWindowVisible, GW_OWNER,
        },
    };

    struct Search {
        pid: u32,
        found: Option<isize>,
    }

    unsafe extern "system" fn each(h: HWND, data: LPARAM) -> BOOL {
        // SAFETY: `data` is the Search passed below, alive for the whole EnumWindows call.
        let search = unsafe { &mut *(data.0 as *mut Search) };
        // SAFETY: read-only queries on a window handle EnumWindows gave us.
        unsafe {
            let mut owner = 0;
            GetWindowThreadProcessId(h, Some(&mut owner));
            let top_level = GetWindow(h, GW_OWNER).map(|o| o.is_invalid()).unwrap_or(true);
            if owner == search.pid && IsWindowVisible(h).as_bool() && top_level && GetWindowTextLengthW(h) > 0
            {
                search.found = Some(h.0 as isize);
                return false.into();
            }
        }
        true.into()
    }

    /// A visible, titled top-level window of this process.
    pub fn window_of(pid: u32) -> Option<i64> {
        let mut search = Search { pid, found: None };
        // SAFETY: `each` only touches the Search behind `data`, which outlives this call.
        unsafe {
            let _ = EnumWindows(Some(each), LPARAM(&mut search as *mut Search as isize));
        }
        search.found.map(|h| h as i64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn costs_follow_the_model() {
        let opus = cost("claude-opus-5-5", 1_000_000, 100_000, 0, 0);
        assert!((opus - 22.5).abs() < 1e-9);
        let haiku = cost("claude-haiku-5-5", 1_000_000, 0, 1_000_000, 0);
        assert!((haiku - 1.1).abs() < 1e-9);
        assert_eq!(prices("something-new"), prices("claude-sonnet-5-5"));
    }

    #[test]
    fn sums_tokens_per_session_for_a_day() {
        let mut conn = Connection::open_in_memory().unwrap();
        crate::db::migrate_for_tests(&mut conn);
        for (id, session, day, model, input, output) in [
            ("m1", "a", "2026-10-14", "claude-opus-5-5", 1000, 200),
            ("m2", "a", "2026-10-14", "claude-haiku-5-5", 500, 50),
            ("m3", "b", "2026-10-14", "claude-sonnet-5-5", 10, 10),
            ("m4", "a", "2026-10-13", "claude-opus-5-5", 9999, 9999),
        ] {
            conn.execute(
                "INSERT INTO claude_messages (message_id, session_id, ts, day, hour, model, input_tokens, output_tokens, cache_read, cache_creation)
                 VALUES (?1, ?2, 0, ?3, 0, ?4, ?5, ?6, 100, 0)",
                rusqlite::params![id, session, day, model, input, output],
            )
            .unwrap();
        }
        let usage = usage_for_day(&conn, "2026-10-14").unwrap();
        assert_eq!(usage.len(), 2);
        assert_eq!(usage[0].session_id, "a");
        assert_eq!(usage[0].input_tokens, 1500);
        assert_eq!(usage[0].cache_tokens, 200);
    }
}

/// Folders Claude Code has worked in, most recent first (from the transcript index).
pub fn projects(conn: &Connection, limit: usize) -> AppResult<Vec<String>> {
    let mut stmt = conn.prepare(
        "SELECT cwd FROM claude_sessions WHERE cwd != '' GROUP BY cwd ORDER BY MAX(last_ts) DESC LIMIT ?1",
    )?;
    let rows = stmt.query_map([limit as i64], |r| r.get::<_, String>(0))?;
    // The same folder can be stored as "c:\…" and "C:\…": keep the first (newest) spelling.
    let mut seen = std::collections::HashSet::new();
    Ok(rows
        .filter_map(Result::ok)
        .filter(|cwd| seen.insert(cwd.trim_end_matches(['\\', '/']).to_lowercase()))
        .filter(|cwd| std::path::Path::new(cwd).is_dir())
        .collect())
}

/// Windows Terminal reads `;` as "new tab": escape it inside the task text.
pub fn terminal_safe(prompt: &str) -> String {
    prompt
        .replace(';', "\\;")
        .replace(['\r', '\n'], " ")
        .trim()
        .to_string()
}

/// Open a terminal in `cwd` running Claude Code on `prompt` (Windows Terminal when it's there).
pub fn launch(cwd: &str, prompt: &str) -> AppResult<()> {
    if !std::path::Path::new(cwd).is_dir() {
        return Err(AppError::InvalidInput(
            "That folder doesn't exist any more".into(),
        ));
    }
    let claude = crate::ask::service::claude_exe()
        .ok_or_else(|| AppError::Platform("Claude Code isn't installed (claude.exe not found)".into()))?;
    let prompt = terminal_safe(prompt);
    let mut wt = std::process::Command::new("wt.exe");
    wt.args(["-d", cwd]).arg(&claude);
    if !prompt.is_empty() {
        wt.arg(&prompt);
    }
    if wt.spawn().is_ok() {
        return Ok(());
    }
    // No Windows Terminal: a plain console window.
    let mut cmd = std::process::Command::new("cmd.exe");
    cmd.args(["/c", "start", "", "/D", cwd]).arg(&claude);
    if !prompt.is_empty() {
        cmd.arg(prompt.replace("\\;", ";"));
    }
    cmd.spawn()
        .map(|_| ())
        .map_err(|e| AppError::Platform(format!("could not open a terminal: {e}")))
}

#[cfg(test)]
mod launch_tests {
    use super::*;

    #[test]
    fn escapes_terminal_separators() {
        assert_eq!(terminal_safe("fix tests; then lint\n"), "fix tests\\; then lint");
    }

    #[test]
    fn lists_existing_project_folders_newest_first() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE claude_sessions (session_id TEXT PRIMARY KEY, cwd TEXT, first_ts INTEGER, last_ts INTEGER);",
        )
        .unwrap();
        let here = std::env::temp_dir().to_string_lossy().into_owned();
        conn.execute(
            "INSERT INTO claude_sessions VALUES ('a', ?1, 1, 5), ('b', 'Z:\\gone\\nowhere', 1, 9), ('c', '', 1, 7)",
            [&here],
        )
        .unwrap();
        assert_eq!(projects(&conn, 10).unwrap(), vec![here.clone()]);
        // The same folder spelled with another case is listed once.
        conn.execute(
            "INSERT INTO claude_sessions VALUES ('d', ?1, 1, 2)",
            [here.to_uppercase()],
        )
        .unwrap();
        assert_eq!(projects(&conn, 10).unwrap().len(), 1);
    }
}
