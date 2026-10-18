//! "Slide aside": when an app you've listed (or any browser, if that's on) is maximized under
//! the notch, the notch moves to the right end of the screen, next to the window buttons,
//! where the tab strip is usually empty, and comes back when that app isn't in front.

use std::{collections::HashMap, sync::Mutex};

use tauri::{AppHandle, Manager};

/// Logical px kept clear at the right end (the window's minimize / maximize / close buttons).
const CLEAR_OF_BUTTONS: f64 = 150.0;

/// Browsers for "slide aside for any maximized browser".
const BROWSERS: &[&str] = &[
    "chrome.exe",
    "msedge.exe",
    "firefox.exe",
    "brave.exe",
    "opera.exe",
    "vivaldi.exe",
    "arc.exe",
    "zen.exe",
];

/// Islands that are aside right now (by window label).
static ASIDE: Mutex<Option<HashMap<String, bool>>> = Mutex::new(None);

pub fn is_aside(label: &str) -> bool {
    ASIDE
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .as_ref()
        .and_then(|m| m.get(label).copied())
        .unwrap_or(false)
}

/// The horizontal offset (logical px from center) that puts an island of `island_width` at the
/// right end of a work area `work_width` wide, clear of the window buttons.
pub fn aside_offset(work_width: f64, island_width: f64) -> i32 {
    (work_width / 2.0 - CLEAR_OF_BUTTONS - island_width / 2.0)
        .max(0.0)
        .round() as i32
}

/// Should the notch move for this app? `exe` is the file name, lowercase.
pub fn wants_aside(exe: &str, listed: &[String], browsers: bool) -> bool {
    listed.iter().any(|l| l.trim().eq_ignore_ascii_case(exe)) || (browsers && BROWSERS.contains(&exe))
}

/// Look at the window in front and move islands aside (or back) as needed.
#[cfg(windows)]
pub fn refresh(app: &AppHandle) {
    use windows::Win32::{
        Graphics::Gdi::{MonitorFromWindow, MONITOR_DEFAULTTONEAREST},
        UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowThreadProcessId, IsZoomed},
    };

    let state = app.state::<crate::state::AppState>();
    let (listed, browsers) = {
        let settings = state.settings.lock().unwrap_or_else(|e| e.into_inner());
        (
            settings.appearance.slide_aside.clone(),
            settings.appearance.slide_aside_browsers,
        )
    };
    // SAFETY: plain queries on the foreground window.
    let (front, pid, zoomed) = unsafe {
        let h = GetForegroundWindow();
        let mut pid = 0;
        GetWindowThreadProcessId(h, Some(&mut pid));
        (h, pid, IsZoomed(h).as_bool())
    };
    // Our own windows in front (the notch itself) change nothing.
    if pid == std::process::id() {
        return;
    }
    let exe = crate::clipboard::win::exe_of(pid)
        .map(|p| crate::clipboard::exe_key(&p))
        .unwrap_or_default();
    let applies = (!listed.is_empty() || browsers) && zoomed && wants_aside(&exe, &listed, browsers);
    // SAFETY: plain query.
    let front_monitor = unsafe { MonitorFromWindow(front, MONITOR_DEFAULTTONEAREST) };

    for window in super::islands::all(app) {
        let on_screen = window.hwnd().is_ok_and(|h| {
            // SAFETY: plain query on our own island window.
            let mine =
                unsafe { MonitorFromWindow(windows::Win32::Foundation::HWND(h.0), MONITOR_DEFAULTTONEAREST) };
            mine == front_monitor
        });
        let want = applies && on_screen;
        let was = ASIDE
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get_or_insert_with(HashMap::new)
            .insert(window.label().to_string(), want)
            .unwrap_or(false);
        let changed = was != want;
        if changed {
            let shell = app.state::<super::ShellState>();
            let size = shell.window_size(window.label());
            let _ = super::place(&window, size);
        }
    }
}

#[cfg(not(windows))]
pub fn refresh(_app: &AppHandle) {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sits_at_the_right_end_clear_of_the_buttons() {
        // 1920 wide: center at 960; a 200 wide bubble ends 150 from the right edge.
        assert_eq!(aside_offset(1920.0, 200.0), 1920 / 2 - 150 - 100);
        // A tiny screen never pushes it left of center.
        assert_eq!(aside_offset(300.0, 400.0), 0);
    }

    #[test]
    fn listed_apps_and_browsers() {
        let listed = vec!["Code.exe".to_string()];
        assert!(wants_aside("code.exe", &listed, false));
        assert!(!wants_aside("chrome.exe", &listed, false));
        assert!(wants_aside("chrome.exe", &listed, true));
        assert!(!wants_aside("notepad.exe", &[], true));
    }
}
