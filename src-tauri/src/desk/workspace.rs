//! Workspaces: every app window's program, title and place, saved under a name and put back
//! later (programs that aren't open are started, then their windows are moved into place).
//! "Last session" is the same thing kept up to date in the background, so after a restart
//! Overline can offer to reopen what you had.

use rusqlite::{params, Connection, OptionalExtension};

use super::place::{SavedWindow, Workspace};
use crate::error::AppResult;

/// The rolling snapshot's name (hidden from the list).
pub const LAST_SESSION: &str = "__last";

pub fn save(conn: &Connection, ws: &Workspace) -> AppResult<()> {
    conn.execute(
        "INSERT INTO workspaces (name, data, saved_at) VALUES (?1, ?2, ?3)
         ON CONFLICT(name) DO UPDATE SET data = excluded.data, saved_at = excluded.saved_at",
        params![
            ws.name,
            serde_json::to_string(&ws.windows)
                .map_err(|e| crate::error::AppError::Platform(e.to_string()))?,
            ws.saved_at
        ],
    )?;
    Ok(())
}

pub fn get(conn: &Connection, name: &str) -> AppResult<Option<Workspace>> {
    let row = conn
        .query_row(
            "SELECT name, data, saved_at FROM workspaces WHERE name = ?1",
            params![name],
            |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, i64>(2)?,
                ))
            },
        )
        .optional()?;
    Ok(row.map(|(name, data, saved_at)| Workspace {
        name,
        saved_at,
        windows: serde_json::from_str(&data).unwrap_or_default(),
    }))
}

/// Saved workspaces, newest first (not the rolling last session).
pub fn list(conn: &Connection) -> AppResult<Vec<Workspace>> {
    let mut stmt =
        conn.prepare("SELECT name, data, saved_at FROM workspaces WHERE name != ?1 ORDER BY saved_at DESC")?;
    let rows = stmt.query_map(params![LAST_SESSION], |r| {
        Ok((
            r.get::<_, String>(0)?,
            r.get::<_, String>(1)?,
            r.get::<_, i64>(2)?,
        ))
    })?;
    Ok(rows
        .filter_map(Result::ok)
        .map(|(name, data, saved_at)| Workspace {
            name,
            saved_at,
            windows: serde_json::from_str(&data).unwrap_or_default(),
        })
        .collect())
}

pub fn delete(conn: &Connection, name: &str) -> AppResult<()> {
    conn.execute("DELETE FROM workspaces WHERE name = ?1", params![name])?;
    Ok(())
}

/// Saved windows whose program isn't running now (what "reopen last session" would start).
pub fn missing(saved: &[SavedWindow], running_exes: &[String]) -> Vec<SavedWindow> {
    let running: std::collections::HashSet<String> = running_exes
        .iter()
        .map(|e| crate::clipboard::exe_key(e))
        .collect();
    let mut seen = std::collections::HashSet::new();
    saved
        .iter()
        .filter(|w| super::place::relaunchable(&w.exe))
        .filter(|w| {
            let key = crate::clipboard::exe_key(&w.exe);
            !running.contains(&key) && seen.insert(key)
        })
        .cloned()
        .collect()
}

#[cfg(windows)]
pub mod win {
    use std::time::{Duration, Instant};

    use windows::Win32::UI::WindowsAndMessaging::GetWindowThreadProcessId;

    use super::super::place::{self, SavedWindow};

    /// How long to wait for a started program's window.
    const WAIT_FOR_WINDOW: Duration = Duration::from_secs(10);

    fn exe_of_window(id: i64) -> Option<String> {
        let mut pid = 0;
        // SAFETY: plain query on a window handle.
        unsafe { GetWindowThreadProcessId(place::win::hwnd(id), Some(&mut pid)) };
        crate::clipboard::win::exe_of(pid)
    }

    /// Every app window now: program, title, place.
    pub fn capture() -> Vec<SavedWindow> {
        crate::desk::win::list()
            .into_iter()
            .filter_map(|w| {
                let exe = exe_of_window(w.id)?;
                let h = place::win::hwnd(w.id);
                let f = place::win::frame(h)?;
                Some(SavedWindow {
                    exe,
                    title: w.title,
                    rect: [f.left, f.top, f.right - f.left, f.bottom - f.top],
                    maximized: place::win::maximized(h),
                })
            })
            .collect()
    }

    /// Open windows as (window id, exe, title).
    fn open_windows() -> Vec<(i64, String, String)> {
        crate::desk::win::list()
            .into_iter()
            .filter_map(|w| Some((w.id, exe_of_window(w.id)?, w.title)))
            .collect()
    }

    /// Put a workspace back. Returns (windows placed, programs started).
    pub fn restore(saved: &[SavedWindow]) -> (usize, usize) {
        let mut started = 0;
        let mut open = open_windows();
        let pairs: Vec<(String, String)> = open.iter().map(|(_, e, t)| (e.clone(), t.clone())).collect();
        let matches = place::match_windows(saved, &pairs);
        // Start what's missing (each program once), then wait for their windows.
        let mut to_start: Vec<&str> = Vec::new();
        for (w, m) in saved.iter().zip(&matches) {
            if m.is_none() && place::relaunchable(&w.exe) && !to_start.contains(&w.exe.as_str()) {
                to_start.push(&w.exe);
            }
        }
        for exe in &to_start {
            if std::process::Command::new(exe).spawn().is_ok() {
                started += 1;
            }
        }
        if !to_start.is_empty() {
            let deadline = Instant::now() + WAIT_FOR_WINDOW;
            while Instant::now() < deadline {
                std::thread::sleep(Duration::from_millis(500));
                open = open_windows();
                let have: Vec<String> = open
                    .iter()
                    .map(|(_, e, _)| crate::clipboard::exe_key(e))
                    .collect();
                if to_start
                    .iter()
                    .all(|e| have.contains(&crate::clipboard::exe_key(e)))
                {
                    // Give fresh windows a moment to finish opening before moving them.
                    std::thread::sleep(Duration::from_millis(700));
                    break;
                }
            }
        }
        let pairs: Vec<(String, String)> = open.iter().map(|(_, e, t)| (e.clone(), t.clone())).collect();
        let matches = place::match_windows(saved, &pairs);
        let mut placed = 0;
        for (w, m) in saved.iter().zip(matches) {
            let Some(j) = m else { continue };
            let h = place::win::hwnd(open[j].0);
            let [x, y, width, height] = w.rect;
            if place::win::place(h, (x, y, width, height)).is_ok() {
                if w.maximized {
                    place::win::maximize(h);
                }
                placed += 1;
            }
        }
        (placed, started)
    }

    /// Programs running now (for "what's missing from last session").
    pub fn running_exes() -> Vec<String> {
        open_windows().into_iter().map(|(_, e, _)| e).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(include_str!("../db/migrations/016_workspaces.sql"))
            .unwrap();
        conn
    }

    fn window(exe: &str) -> SavedWindow {
        SavedWindow {
            exe: exe.into(),
            title: "t".into(),
            rect: [1, 2, 3, 4],
            maximized: true,
        }
    }

    #[test]
    fn saves_lists_and_replaces_by_name() {
        let conn = db();
        let ws = |name: &str, at: i64| Workspace {
            name: name.into(),
            saved_at: at,
            windows: vec![window("C:\\a.exe")],
        };
        save(&conn, &ws("Work", 1)).unwrap();
        save(&conn, &ws("Study", 2)).unwrap();
        save(&conn, &ws(LAST_SESSION, 3)).unwrap();
        save(&conn, &ws("Work", 4)).unwrap();
        let names: Vec<_> = list(&conn).unwrap().into_iter().map(|w| w.name).collect();
        assert_eq!(names, ["Work", "Study"]);
        assert_eq!(get(&conn, "Work").unwrap().unwrap().windows[0].rect, [1, 2, 3, 4]);
        delete(&conn, "Work").unwrap();
        assert!(get(&conn, "Work").unwrap().is_none());
    }

    #[test]
    fn missing_lists_each_program_once_and_skips_running_ones() {
        let saved = [
            window("C:\\x\\Code.exe"),
            window("C:\\x\\Code.exe"),
            window("C:\\y\\spotify.exe"),
            window("C:\\Windows\\explorer.exe"),
        ];
        let running = vec!["c:\\x\\code.exe".to_string()];
        let m = missing(&saved, &running);
        assert_eq!(m.len(), 1);
        assert!(m[0].exe.ends_with("spotify.exe"));
    }
}
