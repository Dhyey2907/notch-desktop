//! Workspaces: every app window's program, title and place, saved under a name and put back
//! later (programs that aren't open are started, then their windows are moved into place).
//! "Last session" is the same thing kept up to date in the background, so after a restart
//! Overline can offer to reopen what you had.

use rusqlite::{params, Connection, OptionalExtension};

use super::place::{OpenApp, SavedWindow, Workspace};
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

/// Saved windows whose app isn't open now (what "reopen last session" would start), each app
/// once. `running`: the apps open now.
pub fn missing(saved: &[SavedWindow], running: &[OpenApp]) -> Vec<SavedWindow> {
    let mut seen = std::collections::HashSet::new();
    saved
        .iter()
        .filter(|w| super::place::relaunchable(w))
        .filter(|w| !running.iter().any(|o| w.is(o)) && seen.insert(w.identity()))
        .cloned()
        .collect()
}

/// What to start to bring back the missing windows: each app once, never one that's open.
pub fn to_start(saved: &[SavedWindow], matches: &[Option<usize>], running: &[OpenApp]) -> Vec<SavedWindow> {
    let unmatched: Vec<SavedWindow> = saved
        .iter()
        .zip(matches)
        .filter(|(_, m)| m.is_none())
        .map(|(w, _)| w.clone())
        .collect();
    missing(&unmatched, running)
}

/// A saved window's name for the "Reopen …" card: a web or Store app by its window title.
pub fn label(w: &SavedWindow) -> String {
    if w.app_id.is_some() && !w.title.trim().is_empty() {
        w.title.split(" - ").next().unwrap_or(&w.title).trim().to_string()
    } else {
        crate::clipboard::app_label(&w.exe)
    }
}

#[cfg(windows)]
pub mod win {
    use std::time::{Duration, Instant};

    use windows::Win32::UI::WindowsAndMessaging::GetWindowThreadProcessId;

    use super::super::place::{self, OpenApp, SavedWindow};

    /// How long to wait for a started program's window.
    const WAIT_FOR_WINDOW: Duration = Duration::from_secs(10);

    fn exe_of_window(id: i64) -> Option<String> {
        let mut pid = 0;
        // SAFETY: plain query on a window handle.
        unsafe { GetWindowThreadProcessId(place::win::hwnd(id), Some(&mut pid)) };
        crate::clipboard::win::exe_of(pid)
    }

    /// Every app window now: program, app id, title, place.
    pub fn capture() -> Vec<SavedWindow> {
        crate::desk::win::list()
            .into_iter()
            .filter_map(|w| {
                let exe = exe_of_window(w.id)?;
                let h = place::win::hwnd(w.id);
                let minimized = place::win::minimized(h);
                let f = if minimized {
                    place::win::normal_rect(h)?
                } else {
                    place::win::frame(h)?
                };
                Some(SavedWindow {
                    exe,
                    title: w.title,
                    rect: [f.left, f.top, f.right - f.left, f.bottom - f.top],
                    maximized: place::win::maximized(h),
                    minimized,
                    app_id: place::win::app_id(h),
                })
            })
            .collect()
    }

    struct Open {
        id: i64,
        app: OpenApp,
        title: String,
    }

    /// Open windows with which app each one is.
    fn open_windows() -> Vec<Open> {
        crate::desk::win::list()
            .into_iter()
            .filter_map(|w| {
                let exe = exe_of_window(w.id)?;
                let app_id = place::win::app_id(place::win::hwnd(w.id));
                Some(Open {
                    id: w.id,
                    app: OpenApp::new(&exe, app_id.as_deref()),
                    title: w.title,
                })
            })
            .collect()
    }

    fn pairs(open: &[Open]) -> Vec<(OpenApp, String)> {
        open.iter().map(|o| (o.app.clone(), o.title.clone())).collect()
    }

    fn start(w: &SavedWindow) -> bool {
        match &w.app_id {
            // Store and web apps start through their app id (Netflix opens as Netflix).
            Some(id) => std::process::Command::new("explorer.exe")
                .arg(format!("shell:AppsFolder\\{id}"))
                .spawn()
                .is_ok(),
            None => std::process::Command::new(&w.exe).spawn().is_ok(),
        }
    }

    /// Put a workspace back. Returns (windows placed, programs started).
    /// Put a workspace back. Returns (windows placed, apps started).
    pub fn restore(saved: &[SavedWindow]) -> (usize, usize) {
        let mut open = open_windows();
        let matches = place::match_windows(saved, &pairs(&open));
        // Start only apps with no window open at all (never a second Edge next to Netflix),
        // each once, then wait for their windows.
        let running: Vec<OpenApp> = open.iter().map(|o| o.app.clone()).collect();
        let starting = super::to_start(saved, &matches, &running);
        let started = starting.iter().filter(|w| start(w)).count();
        if !starting.is_empty() {
            let deadline = Instant::now() + WAIT_FOR_WINDOW;
            while Instant::now() < deadline {
                std::thread::sleep(Duration::from_millis(500));
                open = open_windows();
                if starting.iter().all(|w| open.iter().any(|o| w.is(&o.app))) {
                    // Give fresh windows a moment to finish opening before moving them.
                    std::thread::sleep(Duration::from_millis(900));
                    open = open_windows();
                    break;
                }
            }
        }
        let matches = place::match_windows(saved, &pairs(&open));
        let targets: Vec<(&SavedWindow, i64)> = saved
            .iter()
            .zip(matches)
            .filter_map(|(w, m)| Some((w, open[m?].id)))
            .collect();
        let put = |w: &SavedWindow, id: i64| {
            let h = place::win::hwnd(id);
            let [x, y, width, height] = w.rect;
            let ok = place::win::place(h, (x, y, width, height)).is_ok();
            if ok && w.maximized {
                place::win::maximize(h);
            } else if ok && w.minimized {
                place::win::minimize(h);
            }
            ok
        };
        let placed = targets.iter().filter(|(w, id)| put(w, *id)).count();
        // Apps that just started often move their window back to where they last had it a
        // moment later: check once more and put back any that wandered off.
        if started > 0 {
            std::thread::sleep(Duration::from_millis(1500));
            for (w, id) in &targets {
                let h = place::win::hwnd(*id);
                let [x, y, width, height] = w.rect;
                if !w.maximized && !w.minimized && !place::win::is_at(h, (x, y, width, height)) {
                    put(w, *id);
                }
            }
        }
        (placed, started)
    }

    /// Apps open now (for "what's missing from last session").
    pub fn running() -> Vec<OpenApp> {
        open_windows().into_iter().map(|o| o.app).collect()
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
            minimized: false,
            app_id: None,
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
        let running = vec![OpenApp::new("c:\\x\\code.exe", None)];
        let m = missing(&saved, &running);
        assert_eq!(m.len(), 1);
        assert!(m[0].exe.ends_with("spotify.exe"));
    }

    #[test]
    fn never_starts_a_browser_that_is_already_open() {
        let mut netflix = window("C:\\e\\msedge.exe");
        netflix.app_id = Some("MSEdge._crx_netflix".into());
        netflix.title = "Netflix".into();
        let browser = window("C:\\e\\msedge.exe");
        // Netflix and an Edge window are open; a second saved Edge window found no match.
        let running = vec![
            OpenApp::new(&netflix.exe, netflix.app_id.as_deref()),
            OpenApp::new(&browser.exe, Some("MSEdge")),
        ];
        let saved = [netflix.clone(), browser.clone(), browser];
        assert!(to_start(&saved, &[Some(0), Some(1), None], &running).is_empty());
        // Nothing open: Netflix starts as Netflix, Edge once.
        let s = to_start(&saved, &[None, None, None], &[]);
        assert_eq!(s.len(), 2);
        assert_eq!(label(&s[0]), "Netflix");
    }
}
