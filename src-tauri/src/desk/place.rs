//! Moving windows around: snap a window into part of its screen (halves, thirds, quarters…),
//! and workspaces: remember where every app's windows were and put them back, starting the
//! apps that aren't open. The math is pure and tested; the Windows calls are below it.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// A part of the screen as fractions of its work area (0–1).
#[derive(Debug, Clone, Copy, PartialEq, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Zone {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

/// A rectangle in physical pixels: left, top, width, height.
pub type Rect = (i32, i32, i32, i32);

/// The zone's rectangle inside a work area. Edges are rounded so neighbouring zones meet.
pub fn zone_rect(work: Rect, z: Zone) -> Rect {
    let clamp = |v: f64| v.clamp(0.0, 1.0);
    let (x0, y0) = (clamp(z.x), clamp(z.y));
    let (x1, y1) = (clamp(z.x + z.w), clamp(z.y + z.h));
    let at = |origin: i32, size: i32, f: f64| origin + (f64::from(size) * f).round() as i32;
    let (left, top) = (at(work.0, work.2, x0), at(work.1, work.3, y0));
    let (right, bottom) = (at(work.0, work.2, x1), at(work.1, work.3, y1));
    (left, top, (right - left).max(1), (bottom - top).max(1))
}

/// One window in a saved workspace.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SavedWindow {
    /// Full path of the program.
    pub exe: String,
    pub title: String,
    /// Left, top, width, height (physical pixels, the visible frame).
    pub rect: [i32; 4],
    pub maximized: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Workspace {
    pub name: String,
    /// Unix seconds.
    #[ts(type = "number")]
    pub saved_at: i64,
    pub windows: Vec<SavedWindow>,
}

/// Programs that can't be started again by path (Store apps run inside a host), or shouldn't be.
pub fn relaunchable(exe: &str) -> bool {
    let key = crate::clipboard::exe_key(exe);
    !matches!(
        key.as_str(),
        "applicationframehost.exe" | "explorer.exe" | "notch-desktop.exe" | "systemsettings.exe" | ""
    ) && !key.starts_with("windowsterminal")
}

/// Which open window (index into `open`) each saved window should use: same program, the same
/// title first, each open window used once. `None`: start the program.
pub fn match_windows(saved: &[SavedWindow], open: &[(String, String)]) -> Vec<Option<usize>> {
    let mut used = vec![false; open.len()];
    let key = |exe: &str| crate::clipboard::exe_key(exe);
    let mut out = vec![None; saved.len()];
    // Exact titles first, so "Inbox – Outlook" goes back to its own spot.
    for (i, s) in saved.iter().enumerate() {
        if let Some(j) =
            (0..open.len()).find(|&j| !used[j] && key(&open[j].0) == key(&s.exe) && open[j].1 == s.title)
        {
            used[j] = true;
            out[i] = Some(j);
        }
    }
    for (i, s) in saved.iter().enumerate() {
        if out[i].is_some() {
            continue;
        }
        if let Some(j) = (0..open.len()).find(|&j| !used[j] && key(&open[j].0) == key(&s.exe)) {
            used[j] = true;
            out[i] = Some(j);
        }
    }
    out
}

#[cfg(windows)]
pub mod win {
    use windows::Win32::{
        Foundation::{HWND, RECT},
        Graphics::{
            Dwm::{DwmGetWindowAttribute, DWMWA_EXTENDED_FRAME_BOUNDS},
            Gdi::{GetMonitorInfoW, MonitorFromWindow, MONITORINFO, MONITOR_DEFAULTTONEAREST},
        },
        UI::WindowsAndMessaging::{
            GetWindowRect, IsIconic, IsZoomed, SetWindowPos, ShowWindow, SWP_NOACTIVATE, SWP_NOZORDER,
            SW_MAXIMIZE, SW_RESTORE,
        },
    };

    use super::Rect;
    use crate::error::{AppError, AppResult};

    pub fn hwnd(id: i64) -> HWND {
        HWND(id as isize as *mut _)
    }

    /// The visible frame (without the invisible resize borders Windows 10/11 adds).
    pub fn frame(h: HWND) -> Option<RECT> {
        let mut r = RECT::default();
        // SAFETY: fills a local RECT of the size given.
        unsafe {
            DwmGetWindowAttribute(
                h,
                DWMWA_EXTENDED_FRAME_BOUNDS,
                (&mut r as *mut RECT).cast(),
                std::mem::size_of::<RECT>() as u32,
            )
        }
        .ok()
        .map(|()| r)
        .or_else(|| {
            let mut w = RECT::default();
            // SAFETY: fills a local RECT.
            unsafe { GetWindowRect(h, &mut w) }.ok().map(|()| w)
        })
    }

    pub fn maximized(h: HWND) -> bool {
        // SAFETY: plain query.
        unsafe { IsZoomed(h).as_bool() }
    }

    /// The work area (screen minus taskbar) of the screen the window is mostly on.
    pub fn work_area(h: HWND) -> Option<Rect> {
        // SAFETY: plain queries filling a local struct.
        unsafe {
            let monitor = MonitorFromWindow(h, MONITOR_DEFAULTTONEAREST);
            let mut info = MONITORINFO {
                cbSize: std::mem::size_of::<MONITORINFO>() as u32,
                ..Default::default()
            };
            GetMonitorInfoW(monitor, &mut info).ok().ok()?;
            let w = info.rcWork;
            Some((w.left, w.top, w.right - w.left, w.bottom - w.top))
        }
    }

    /// Put a window's visible frame at `rect` (restoring it first if it's maximized or minimized).
    pub fn place(h: HWND, rect: Rect) -> AppResult<()> {
        // SAFETY: standard window calls on a live top-level window.
        unsafe {
            if IsZoomed(h).as_bool() || IsIconic(h).as_bool() {
                let _ = ShowWindow(h, SW_RESTORE);
            }
            // The outer rect includes invisible borders: grow by them so the visible part lands
            // exactly on `rect`.
            let mut outer = RECT::default();
            GetWindowRect(h, &mut outer).map_err(|e| AppError::Platform(e.to_string()))?;
            let visible = super::win::frame(h).unwrap_or(outer);
            let (dl, dt) = (visible.left - outer.left, visible.top - outer.top);
            let (dr, db) = (outer.right - visible.right, outer.bottom - visible.bottom);
            SetWindowPos(
                h,
                None,
                rect.0 - dl,
                rect.1 - dt,
                rect.2 + dl + dr,
                rect.3 + dt + db,
                SWP_NOZORDER | SWP_NOACTIVATE,
            )
            .map_err(|e| AppError::Platform(format!("could not move the window: {e}")))
        }
    }

    pub fn maximize(h: HWND) {
        // SAFETY: plain call on a live window.
        unsafe {
            let _ = ShowWindow(h, SW_MAXIMIZE);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const WORK: Rect = (0, 0, 1920, 1040);

    #[test]
    fn halves_meet_in_the_middle() {
        let left = zone_rect(
            WORK,
            Zone {
                x: 0.0,
                y: 0.0,
                w: 0.5,
                h: 1.0,
            },
        );
        let right = zone_rect(
            WORK,
            Zone {
                x: 0.5,
                y: 0.0,
                w: 0.5,
                h: 1.0,
            },
        );
        assert_eq!(left, (0, 0, 960, 1040));
        assert_eq!(right, (960, 0, 960, 1040));
    }

    #[test]
    fn thirds_cover_the_screen_on_a_second_monitor() {
        let work = (-1280, 0, 1280, 1000);
        let third = 1.0 / 3.0;
        let a = zone_rect(
            work,
            Zone {
                x: 0.0,
                y: 0.0,
                w: third,
                h: 1.0,
            },
        );
        let b = zone_rect(
            work,
            Zone {
                x: third,
                y: 0.0,
                w: third,
                h: 1.0,
            },
        );
        let c = zone_rect(
            work,
            Zone {
                x: 2.0 * third,
                y: 0.0,
                w: third,
                h: 1.0,
            },
        );
        assert_eq!(a.0, -1280);
        assert_eq!(a.0 + a.2, b.0);
        assert_eq!(b.0 + b.2, c.0);
        assert_eq!(c.0 + c.2, 0);
    }

    #[test]
    fn zones_are_clamped_to_the_screen() {
        assert_eq!(
            zone_rect(
                WORK,
                Zone {
                    x: 0.9,
                    y: 0.0,
                    w: 0.5,
                    h: 1.0
                }
            ),
            (1728, 0, 192, 1040)
        );
    }

    fn saved(exe: &str, title: &str) -> SavedWindow {
        SavedWindow {
            exe: exe.into(),
            title: title.into(),
            rect: [0, 0, 10, 10],
            maximized: false,
        }
    }

    #[test]
    fn matches_titles_first_then_any_window_of_the_app() {
        let saved = [
            saved("C:\\a\\Code.exe", "notch – Code"),
            saved("C:\\a\\Code.exe", "site – Code"),
            saved("C:\\b\\chrome.exe", "Gmail"),
            saved("C:\\c\\spotify.exe", "Spotify"),
        ];
        let open = vec![
            ("c:\\a\\code.exe".to_string(), "site – Code".to_string()),
            ("C:\\a\\Code.exe".to_string(), "other – Code".to_string()),
            ("C:\\b\\chrome.exe".to_string(), "New Tab".to_string()),
        ];
        assert_eq!(
            match_windows(&saved, &open),
            vec![Some(1), Some(0), Some(2), None]
        );
    }

    #[test]
    fn store_apps_and_explorer_are_not_relaunched() {
        assert!(!relaunchable("C:\\Windows\\System32\\ApplicationFrameHost.exe"));
        assert!(!relaunchable("C:\\Windows\\explorer.exe"));
        assert!(relaunchable(
            "C:\\Program Files\\Google\\Chrome\\Application\\chrome.exe"
        ));
    }
}
