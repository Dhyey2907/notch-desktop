//! "Fade while typing": when you type in a text box just under the resting notch (a browser's
//! address bar, a chat box at the top of a window), the bubble fades so it doesn't cover what
//! you're writing, and comes back a moment after you stop. Typing is noticed as input with a
//! still mouse; where you're typing comes from UI Automation's focused element.

/// A rectangle in physical pixels: left, top, right, bottom.
pub type Rect = (i32, i32, i32, i32);

/// How far below the notch (physical px at 100%) a text box still counts as "under" it.
const BAND: i32 = 140;
/// Sideways slack around the notch.
const SLACK: i32 = 60;

/// Is the text box `field` close enough to the notch `island` to be covered by it?
/// `bottom`: the notch sits on the bottom edge, so look above it instead.
pub fn under_notch(island: Rect, field: Rect, scale: f64, bottom: bool) -> bool {
    let band = (f64::from(BAND) * scale).round() as i32;
    let slack = (f64::from(SLACK) * scale).round() as i32;
    let overlaps_x = field.0 < island.2 + slack && field.2 > island.0 - slack;
    let overlaps_y = if bottom {
        field.3 > island.1 - band && field.1 < island.3
    } else {
        field.1 < island.3 + band && field.3 > island.1
    };
    overlaps_x && overlaps_y
}

#[cfg(windows)]
pub fn start(app: &tauri::AppHandle) {
    use std::time::{Duration, Instant};

    use tauri::Manager;
    use windows::Win32::{
        Foundation::POINT,
        System::Com::{CoCreateInstance, CoInitializeEx, CLSCTX_INPROC_SERVER, COINIT_MULTITHREADED},
        UI::{
            Accessibility::{CUIAutomation, IUIAutomation},
            Input::KeyboardAndMouse::{GetLastInputInfo, LASTINPUTINFO},
            WindowsAndMessaging::{GetCursorPos, GetForegroundWindow, GetWindowThreadProcessId},
        },
    };

    use crate::{events::AppEvent, state::AppState};

    const TICK: Duration = Duration::from_millis(200);
    /// The bubble comes back this long after the last key.
    const HOLD: Duration = Duration::from_millis(1500);

    let app = app.clone();
    std::thread::Builder::new()
        .name("fade-while-typing".into())
        .spawn(move || {
            // SAFETY: COM for this worker thread only; the automation object lives on it.
            let automation: Option<IUIAutomation> = unsafe {
                let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
                CoCreateInstance(&CUIAutomation, None, CLSCTX_INPROC_SERVER).ok()
            };
            let mut last_input = 0u32;
            let mut last_cursor = POINT::default();
            let mut last_key: Option<Instant> = None;
            let mut faded: std::collections::HashMap<String, bool> = std::collections::HashMap::new();
            loop {
                std::thread::sleep(TICK);
                let (on, bottom) = {
                    let s = app
                        .state::<AppState>()
                        .settings
                        .lock()
                        .unwrap_or_else(|e| e.into_inner())
                        .appearance
                        .clone();
                    (
                        s.fade_while_typing && !s.edge.is_side(),
                        s.edge == crate::settings::ScreenEdge::Bottom,
                    )
                };
                if !on {
                    for (label, was) in faded.iter_mut() {
                        if *was {
                            *was = false;
                            app.state::<AppState>().bus.emit(
                                "shell",
                                AppEvent::NotchFade {
                                    label: label.clone(),
                                    on: false,
                                },
                            );
                        }
                    }
                    std::thread::sleep(Duration::from_secs(1));
                    continue;
                }
                // SAFETY: plain queries into local structs.
                let (input, cursor, own_front) = unsafe {
                    let mut info = LASTINPUTINFO {
                        cbSize: std::mem::size_of::<LASTINPUTINFO>() as u32,
                        dwTime: 0,
                    };
                    let _ = GetLastInputInfo(&mut info);
                    let mut p = POINT::default();
                    let _ = GetCursorPos(&mut p);
                    let mut pid = 0;
                    GetWindowThreadProcessId(GetForegroundWindow(), Some(&mut pid));
                    (info.dwTime, p, pid == std::process::id())
                };
                // New input while the mouse stayed still: a key.
                if input != last_input && (cursor.x, cursor.y) == (last_cursor.x, last_cursor.y) && !own_front
                {
                    last_key = Some(Instant::now());
                }
                last_input = input;
                last_cursor = cursor;
                let typing = last_key.is_some_and(|t| t.elapsed() < HOLD);
                // Where the focused text box is (only asked while typing).
                let field = if typing {
                    automation.as_ref().and_then(|a| {
                        // SAFETY: UI Automation calls on this thread's MTA.
                        unsafe {
                            let r = a.GetFocusedElement().ok()?.CurrentBoundingRectangle().ok()?;
                            (r.right > r.left && r.bottom > r.top)
                                .then_some((r.left, r.top, r.right, r.bottom))
                        }
                    })
                } else {
                    None
                };
                for window in super::islands::all(&app) {
                    let label = window.label().to_string();
                    let want = match (
                        field,
                        window.outer_position(),
                        window.outer_size(),
                        window.scale_factor(),
                    ) {
                        (Some(f), Ok(p), Ok(s), Ok(scale)) => {
                            let island = (p.x, p.y, p.x + s.width as i32, p.y + s.height as i32);
                            under_notch(island, f, scale, bottom)
                        }
                        _ => false,
                    };
                    let was = faded.insert(label.clone(), want).unwrap_or(false);
                    if was != want {
                        app.state::<AppState>()
                            .bus
                            .emit("shell", AppEvent::NotchFade { label, on: want });
                    }
                }
            }
        })
        .ok();
}

#[cfg(not(windows))]
pub fn start(_app: &tauri::AppHandle) {}

#[cfg(test)]
mod tests {
    use super::*;

    const ISLAND: Rect = (860, 0, 1060, 24);

    #[test]
    fn a_text_box_right_under_the_notch_counts() {
        // An address bar across the top of a maximized browser.
        assert!(under_notch(ISLAND, (200, 50, 1700, 80), 1.0, false));
        // A box far down the page doesn't.
        assert!(!under_notch(ISLAND, (200, 400, 1700, 430), 1.0, false));
        // A box at the left end of the screen doesn't.
        assert!(!under_notch(ISLAND, (0, 40, 300, 70), 1.0, false));
    }

    #[test]
    fn the_band_scales_and_flips_for_the_bottom_edge() {
        assert!(under_notch(ISLAND, (900, 200, 1000, 230), 1.5, false));
        assert!(!under_notch(ISLAND, (900, 200, 1000, 230), 1.0, false));
        let bottom = (860, 1000, 1060, 1024);
        assert!(under_notch(bottom, (700, 900, 1200, 930), 1.0, true));
        assert!(!under_notch(bottom, (700, 500, 1200, 530), 1.0, true));
    }
}
