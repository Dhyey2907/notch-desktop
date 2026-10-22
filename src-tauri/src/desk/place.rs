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
    /// Left, top, width, height (physical pixels, the visible frame; for a minimized window,
    /// where it sits when it's not minimized).
    pub rect: [i32; 4],
    pub maximized: bool,
    #[serde(default)]
    pub minimized: bool,
    /// The app's own Windows id (e.g. Netflix inside Edge, or a Store app), so it reopens as
    /// that app and not as the program hosting it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub app_id: Option<String>,
}

/// Which app a window is: its own Windows id when it has one, else its program.
pub fn identity(exe: &str, app_id: Option<&str>) -> String {
    match app_id {
        Some(id) if !id.trim().is_empty() => id.trim().to_lowercase(),
        _ => crate::clipboard::exe_key(exe),
    }
}

impl SavedWindow {
    pub fn identity(&self) -> String {
        identity(&self.exe, self.app_id.as_deref())
    }

    /// Whether an open window is this app. Saved before app ids were kept: by program.
    pub fn is(&self, open: &OpenApp) -> bool {
        match &self.app_id {
            Some(_) => open.identity == self.identity(),
            None => open.exe == crate::clipboard::exe_key(&self.exe),
        }
    }
}

/// An open window's app: its identity (see `identity`) and program.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenApp {
    pub identity: String,
    pub exe: String,
}

impl OpenApp {
    pub fn new(exe: &str, app_id: Option<&str>) -> Self {
        Self {
            identity: identity(exe, app_id),
            exe: crate::clipboard::exe_key(exe),
        }
    }
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
pub fn relaunchable(w: &SavedWindow) -> bool {
    // Anything with its own app id can be started through it (Store apps, web apps).
    w.app_id
        .as_deref()
        .is_some_and(|id| !id.trim().is_empty() && !id.contains("notch"))
        || relaunchable_exe(&w.exe)
}

/// A program that can be started by running it again.
pub fn relaunchable_exe(exe: &str) -> bool {
    let key = crate::clipboard::exe_key(exe);
    !matches!(
        key.as_str(),
        "applicationframehost.exe" | "explorer.exe" | "notch-desktop.exe" | "systemsettings.exe" | ""
    ) && !key.starts_with("windowsterminal")
}

/// Which open window (index into `open`) each saved window should use: same app, the same
/// title first, each open window used once. `open` is (app, title). `None`: not open.
pub fn match_windows(saved: &[SavedWindow], open: &[(OpenApp, String)]) -> Vec<Option<usize>> {
    let mut used = vec![false; open.len()];
    let mut out = vec![None; saved.len()];
    // Exact titles first, so "Inbox – Outlook" goes back to its own spot.
    for (i, s) in saved.iter().enumerate() {
        if let Some(j) = (0..open.len()).find(|&j| !used[j] && s.is(&open[j].0) && open[j].1 == s.title) {
            used[j] = true;
            out[i] = Some(j);
        }
    }
    for (i, s) in saved.iter().enumerate() {
        if out[i].is_some() {
            continue;
        }
        if let Some(j) = (0..open.len()).find(|&j| !used[j] && s.is(&open[j].0)) {
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
            GetWindowPlacement, GetWindowRect, IsIconic, IsZoomed, SetWindowPos, ShowWindow, SWP_NOACTIVATE,
            SWP_NOZORDER, SW_MAXIMIZE, SW_MINIMIZE, SW_RESTORE, WINDOWPLACEMENT,
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

    pub fn minimized(h: HWND) -> bool {
        // SAFETY: plain query.
        unsafe { IsIconic(h).as_bool() }
    }

    /// Where a minimized window sits when it's restored (screen coordinates). Windows reports
    /// minimized windows far off screen, so their own frame is no use for putting them back.
    pub fn normal_rect(h: HWND) -> Option<RECT> {
        let mut p = WINDOWPLACEMENT {
            length: std::mem::size_of::<WINDOWPLACEMENT>() as u32,
            ..Default::default()
        };
        // SAFETY: fills a local struct of the size given.
        unsafe { GetWindowPlacement(h, &mut p) }.ok()?;
        let r = p.rcNormalPosition;
        // Workspace coordinates: offset by the work area of the screen it's on.
        let (wx, wy, _, _) = work_area(h).unwrap_or((0, 0, 0, 0));
        let (mx, my) = monitor_origin(h).unwrap_or((0, 0));
        let (dx, dy) = (wx - mx, wy - my);
        Some(RECT {
            left: r.left + dx,
            top: r.top + dy,
            right: r.right + dx,
            bottom: r.bottom + dy,
        })
    }

    fn monitor_origin(h: HWND) -> Option<(i32, i32)> {
        // SAFETY: plain queries filling a local struct.
        unsafe {
            let monitor = MonitorFromWindow(h, MONITOR_DEFAULTTONEAREST);
            let mut info = MONITORINFO {
                cbSize: std::mem::size_of::<MONITORINFO>() as u32,
                ..Default::default()
            };
            GetMonitorInfoW(monitor, &mut info).ok().ok()?;
            Some((info.rcMonitor.left, info.rcMonitor.top))
        }
    }

    /// The app's own Windows id (System.AppUserModel.ID), when the window has one.
    pub fn app_id(h: HWND) -> Option<String> {
        use windows::core::GUID;
        use windows::Win32::{
            Foundation::PROPERTYKEY,
            UI::Shell::PropertiesSystem::{IPropertyStore, SHGetPropertyStoreForWindow},
        };
        const APP_USER_MODEL_ID: PROPERTYKEY = PROPERTYKEY {
            fmtid: GUID::from_u128(0x9F4C2855_9F79_4B39_A8D0_E1D42DE1D5F3),
            pid: 5,
        };
        // SAFETY: COM calls on a live window; the store is released when dropped.
        let id = unsafe {
            let store: IPropertyStore = SHGetPropertyStoreForWindow(h).ok()?;
            store.GetValue(&APP_USER_MODEL_ID).ok()?.to_string()
        };
        let id = id.trim().to_string();
        (!id.is_empty()).then_some(id)
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
            // Twice: moving onto a screen with a different scale makes the window resize itself
            // for the new scale, so the second pass lands it exactly.
            for _ in 0..2 {
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
                .map_err(|e| AppError::Platform(format!("could not move the window: {e}")))?;
            }
            Ok(())
        }
    }

    /// Whether the visible frame is already (within a few px) at `rect`.
    pub fn is_at(h: HWND, rect: Rect) -> bool {
        frame(h).is_some_and(|f| {
            (f.left - rect.0).abs() <= 8
                && (f.top - rect.1).abs() <= 8
                && ((f.right - f.left) - rect.2).abs() <= 8
                && ((f.bottom - f.top) - rect.3).abs() <= 8
        })
    }

    pub fn minimize(h: HWND) {
        // SAFETY: plain call on a live window.
        unsafe {
            let _ = ShowWindow(h, SW_MINIMIZE);
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
            minimized: false,
            app_id: None,
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
        let open = [
            ("c:\\a\\code.exe", "site – Code"),
            ("C:\\a\\Code.exe", "other – Code"),
            ("C:\\b\\chrome.exe", "New Tab"),
        ]
        .map(|(exe, title)| (OpenApp::new(exe, None), title.to_string()));
        assert_eq!(
            match_windows(&saved, &open),
            vec![Some(1), Some(0), Some(2), None]
        );
    }

    #[test]
    fn a_web_app_is_its_own_app_not_its_browser() {
        let mut netflix = saved("C:\\e\\msedge.exe", "Netflix");
        netflix.app_id = Some("MSEdge._crx_netflix".into());
        let mut browser = saved("C:\\e\\msedge.exe", "News - Edge");
        browser.app_id = Some("MSEdge".into());
        // Only Netflix is open: the browser window is not matched to it.
        let open = vec![(
            OpenApp::new("C:\\e\\msedge.exe", Some("MSEdge._crx_netflix")),
            "Netflix".to_string(),
        )];
        assert_eq!(
            match_windows(&[browser, netflix.clone()], &open),
            vec![None, Some(0)]
        );
        assert!(relaunchable(&netflix));
    }

    #[test]
    fn windows_saved_before_app_ids_match_by_program() {
        // An older saved Chrome window has no app id; open Chrome now reports one.
        let old = saved("C:\\b\\chrome.exe", "Gmail");
        let open = vec![(
            OpenApp::new("C:\\b\\chrome.exe", Some("Chrome")),
            "Inbox".to_string(),
        )];
        assert_eq!(match_windows(&[old], &open), vec![Some(0)]);
    }

    #[test]
    fn store_apps_and_explorer_are_not_relaunched() {
        assert!(!relaunchable_exe(
            "C:\\Windows\\System32\\ApplicationFrameHost.exe"
        ));
        assert!(!relaunchable_exe("C:\\Windows\\explorer.exe"));
        assert!(relaunchable_exe(
            "C:\\Program Files\\Google\\Chrome\\Application\\chrome.exe"
        ));
    }
}
