//! Calling the pet out of the notch, and what's happening on the pet's own screen.
//!
//! There is one pet and it is never "placed" anywhere: called out, it walks the taskbar (the
//! pet loop in `service` does the walking) and keeps walking until you send it back.
//!
//! The pet lives on one screen (the one the notch is on) and only that screen matters to it:
//! where you're coding, where a video plays, where you click. Another screen never affects it.

use tauri::{AppHandle, Manager};

use crate::{error::AppResult, state::AppState};

fn called_out(app: &AppHandle) -> bool {
    app.state::<AppState>()
        .settings
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .pet
        .buddy_out
}

fn set_called_out(app: &AppHandle, out: bool) -> AppResult<()> {
    if called_out(app) == out {
        return Ok(());
    }
    let state = app.state::<AppState>();
    let mut next = state.settings.lock().unwrap_or_else(|e| e.into_inner()).clone();
    next.pet.buddy_out = out;
    crate::commands::settings::persist_and_broadcast(&state, next).map(|_| ())
}

/// Call the pet out: it starts walking the taskbar and keeps walking.
pub fn show(app: &AppHandle) -> AppResult<()> {
    set_called_out(app, true)
}

/// Send it back into the notch.
pub fn hide(app: &AppHandle) -> AppResult<()> {
    set_called_out(app, false)
}

/// Where something is, seen from the pet: on its own screen, on another one, or unknown.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Place {
    Here,
    Elsewhere,
    Unknown,
}

impl Place {
    pub fn as_str(self) -> &'static str {
        match self {
            Place::Here => "here",
            Place::Elsewhere => "elsewhere",
            Place::Unknown => "unknown",
        }
    }
}

/// From "does it have a window on the pet's screen / on another screen".
fn place_of(here: bool, elsewhere: bool) -> Place {
    match (here, elsewhere) {
        (true, _) => Place::Here,
        (false, true) => Place::Elsewhere,
        (false, false) => Place::Unknown,
    }
}

/// A window's program is the app that's playing: "chrome" for the source "Chrome" or
/// "Chrome._crx_…", "spotify" for "SpotifyAB.SpotifyMusic_…!Spotify". Short names are skipped
/// so they can't match by accident.
fn plays_from(source_id: &str, exe_stem: &str) -> bool {
    exe_stem.len() >= 3 && source_id.to_lowercase().contains(&exe_stem.to_lowercase())
}

#[cfg(windows)]
mod win {
    use tauri::{AppHandle, Manager};
    use windows::core::BOOL;
    use windows::Win32::{
        Foundation::{HWND, LPARAM, POINT},
        Graphics::Gdi::{MonitorFromPoint, MonitorFromWindow, HMONITOR, MONITOR_DEFAULTTONEAREST},
        UI::WindowsAndMessaging::{
            EnumWindows, GetCursorPos, GetForegroundWindow, GetWindowThreadProcessId, IsIconic,
            IsWindowVisible,
        },
    };

    /// The screen the pet walks on: the one the notch is on.
    pub fn pet_monitor(app: &AppHandle) -> Option<HMONITOR> {
        let notch = app.get_webview_window(crate::shell::MAIN_WINDOW)?.hwnd().ok()?;
        // SAFETY: a live top-level window of this process; this only asks which monitor it's on.
        Some(unsafe { MonitorFromWindow(HWND(notch.0), MONITOR_DEFAULTTONEAREST) })
    }

    pub fn front_window() -> (HWND, u32) {
        // SAFETY: read-only queries of the foreground window.
        unsafe {
            let front = GetForegroundWindow();
            let mut pid = 0;
            GetWindowThreadProcessId(front, Some(&mut pid));
            (front, pid)
        }
    }

    pub fn monitor_of(window: HWND) -> HMONITOR {
        // SAFETY: only asks which monitor a window is on (nearest one for an invalid handle).
        unsafe { MonitorFromWindow(window, MONITOR_DEFAULTTONEAREST) }
    }

    pub fn cursor_point() -> Option<POINT> {
        let mut p = POINT::default();
        // SAFETY: writes the cursor position into a POINT we own.
        unsafe { GetCursorPos(&mut p) }.ok()?;
        Some(p)
    }

    pub fn cursor_monitor() -> Option<HMONITOR> {
        let p = cursor_point()?;
        // SAFETY: only asks which monitor a point is on.
        Some(unsafe { MonitorFromPoint(p, MONITOR_DEFAULTTONEAREST) })
    }

    /// Every visible, not minimized top-level window with its process id.
    pub fn open_windows() -> Vec<(HWND, u32)> {
        unsafe extern "system" fn collect(window: HWND, data: LPARAM) -> BOOL {
            // SAFETY: `data` is the Vec passed below, alive for the whole EnumWindows call.
            let found = unsafe { &mut *(data.0 as *mut Vec<(HWND, u32)>) };
            // SAFETY: read-only queries on a window handle EnumWindows just gave us.
            unsafe {
                if IsWindowVisible(window).as_bool() && !IsIconic(window).as_bool() {
                    let mut pid = 0;
                    GetWindowThreadProcessId(window, Some(&mut pid));
                    found.push((window, pid));
                }
            }
            true.into()
        }
        let mut found: Vec<(HWND, u32)> = Vec::new();
        // SAFETY: `collect` only touches the Vec behind `data`, which outlives this call.
        unsafe {
            let _ = EnumWindows(Some(collect), LPARAM(&mut found as *mut _ as isize));
        }
        found
    }
}

/// Where the mouse is (physical pixels).
pub fn cursor() -> Option<(i32, i32)> {
    #[cfg(windows)]
    {
        win::cursor_point().map(|p| (p.x, p.y))
    }
    #[cfg(not(windows))]
    None
}

/// The mouse is on the pet's own screen (true when that can't be told).
pub fn cursor_on_pet_screen(app: &AppHandle) -> bool {
    #[cfg(windows)]
    {
        match (win::pet_monitor(app), win::cursor_monitor()) {
            (Some(pet), Some(cursor)) => pet == cursor,
            _ => true,
        }
    }
    #[cfg(not(windows))]
    {
        let _ = app;
        true
    }
}

/// The app in front is on the pet's own screen (true when that can't be told).
pub fn front_on_pet_screen(app: &AppHandle) -> bool {
    #[cfg(windows)]
    {
        win::pet_monitor(app).is_none_or(|pet| win::monitor_of(win::front_window().0) == pet)
    }
    #[cfg(not(windows))]
    {
        let _ = app;
        true
    }
}

/// Where the app that's playing shows its windows: on the pet's screen, only on another
/// screen, or unknown (no window found for it, or nothing playing).
pub fn media_place(app: &AppHandle) -> Place {
    let source = crate::media::playing_source();
    if source.is_empty() {
        return Place::Unknown;
    }
    #[cfg(windows)]
    {
        let Some(pet) = win::pet_monitor(app) else {
            return Place::Unknown;
        };
        let mut stems: std::collections::HashMap<u32, Option<String>> = std::collections::HashMap::new();
        let (mut here, mut elsewhere) = (false, false);
        for (window, pid) in win::open_windows() {
            let stem = stems.entry(pid).or_insert_with(|| {
                crate::clipboard::win::exe_of(pid).and_then(|exe| {
                    std::path::Path::new(&exe)
                        .file_stem()
                        .map(|s| s.to_string_lossy().into_owned())
                })
            });
            if stem.as_deref().is_some_and(|stem| plays_from(&source, stem)) {
                if win::monitor_of(window) == pet {
                    here = true;
                } else {
                    elsewhere = true;
                }
            }
        }
        place_of(here, elsewhere)
    }
    #[cfg(not(windows))]
    {
        let _ = app;
        Place::Unknown
    }
}

/// Terminals count as coding too (that's where Claude Code and Codex run).
const TERMINALS: &[&str] = &[
    "windowsterminal",
    "wezterm-gui",
    "alacritty",
    "warp",
    "hyper",
    "tabby",
];

/// Whether you're coding (a code editor or a terminal is the app in front), and where:
/// `"here"` on the pet's own screen (the one the notch is on), `"elsewhere"` on another
/// screen, `"none"` when you aren't.
pub fn coding_in_front(app: &AppHandle) -> &'static str {
    #[cfg(windows)]
    {
        let (front, pid) = win::front_window();
        let Some(exe) = crate::clipboard::win::exe_of(pid) else {
            return "none";
        };
        let stem = std::path::Path::new(&exe)
            .file_stem()
            .map(|s| s.to_string_lossy().to_lowercase())
            .unwrap_or_default();
        if !(crate::developer::is_editor(&stem) || TERMINALS.contains(&stem.as_str())) {
            return "none";
        }
        let same_screen = win::pet_monitor(app).is_none_or(|pet| win::monitor_of(front) == pet);
        where_coding(true, same_screen)
    }
    #[cfg(not(windows))]
    {
        let _ = app;
        "none"
    }
}

fn where_coding(coding: bool, same_screen: bool) -> &'static str {
    match (coding, same_screen) {
        (false, _) => "none",
        (true, true) => "here",
        (true, false) => "elsewhere",
    }
}

#[cfg(test)]
mod tests {
    use super::{place_of, plays_from, where_coding, Place};

    #[test]
    fn coding_counts_as_here_only_on_the_pets_own_screen() {
        assert_eq!(where_coding(true, true), "here");
        assert_eq!(where_coding(true, false), "elsewhere");
        assert_eq!(where_coding(false, true), "none");
    }

    #[test]
    fn a_video_with_windows_only_on_another_screen_is_elsewhere() {
        assert_eq!(place_of(false, true), Place::Elsewhere);
        // A window on the pet's screen too (two browser windows): it's here.
        assert_eq!(place_of(true, true), Place::Here);
        assert_eq!(place_of(true, false), Place::Here);
        // No window found for the app that's playing: can't tell.
        assert_eq!(place_of(false, false), Place::Unknown);
    }

    #[test]
    fn matches_a_window_to_the_app_that_is_playing() {
        assert!(plays_from("Chrome", "chrome"));
        assert!(plays_from("Chrome._crx_abcdef", "chrome"));
        assert!(plays_from("MSEdge", "msedge"));
        assert!(plays_from(
            "SpotifyAB.SpotifyMusic_zpdnekdrzrea0!Spotify",
            "Spotify"
        ));
        assert!(!plays_from("Chrome", "code"));
        assert!(!plays_from("MSEdge", "explorer"));
        // Too short to mean anything.
        assert!(!plays_from("Chrome", "ch"));
    }
}
