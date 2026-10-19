//! Away screen: when you step away from the PC (or ask for it), a dim, glanceable view fills
//! your second monitor (or every monitor): a big clock, next meeting, weather, what's playing,
//! unread mail. Any key or mouse movement takes it away. It isn't a lock: Windows' own lock
//! screen can't show other apps, so this is the "while you're away" version.

use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, PhysicalPosition, PhysicalSize, WebviewUrl, WebviewWindowBuilder};
use ts_rs::TS;

use crate::error::AppResult;

pub const PREFIX: &str = "away-";
/// Input within this long (ms) counts as "you're back".
const BACK_MS: u64 = 800;
/// Shown by hand: ignore the input that asked for it.
const GRACE: Duration = Duration::from_millis(1500);

static SHOWING: AtomicBool = AtomicBool::new(false);

/// Settings → Look → Away screen.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export)]
pub struct AwaySettings {
    /// Show after this many idle minutes (0 = only when asked, from the palette).
    pub after_idle_min: u32,
    /// Every monitor instead of only the others (with one monitor, it's that one).
    pub all_screens: bool,
}

impl AwaySettings {
    pub fn valid(&self) -> bool {
        self.after_idle_min <= 240
    }
}

/// Which monitors (by index) show it: the non-primary ones, or all; one monitor → that one.
pub fn targets(count: usize, primary: Option<usize>, all: bool) -> Vec<usize> {
    if all || count <= 1 {
        return (0..count).collect();
    }
    (0..count).filter(|i| Some(*i) != primary).collect()
}

/// Should it come up by itself now?
pub fn due(s: &AwaySettings, idle_ms: u64, locked: bool, showing: bool) -> bool {
    s.after_idle_min > 0 && !locked && !showing && idle_ms >= u64::from(s.after_idle_min) * 60_000
}

pub fn showing() -> bool {
    SHOWING.load(Ordering::SeqCst)
}

pub fn hide(app: &AppHandle) {
    SHOWING.store(false, Ordering::SeqCst);
    for (label, w) in app.webview_windows() {
        if label.starts_with(PREFIX) {
            let _ = w.close();
        }
    }
}

/// Bring it up; it goes away on the next key or mouse movement.
#[cfg(windows)]
pub fn show(app: &AppHandle, by_hand: bool) -> AppResult<()> {
    if SHOWING.swap(true, Ordering::SeqCst) {
        return Ok(());
    }
    let all = app
        .state::<crate::state::AppState>()
        .settings
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .away
        .all_screens;
    let monitors = app.available_monitors()?;
    let primary_pos = app.primary_monitor()?.map(|m| *m.position());
    let primary = monitors.iter().position(|m| Some(*m.position()) == primary_pos);
    for i in targets(monitors.len(), primary, all) {
        let m = &monitors[i];
        let w = WebviewWindowBuilder::new(
            app,
            format!("{PREFIX}{i}"),
            WebviewUrl::App("index.html#away".into()),
        )
        .title("Overline away screen")
        .decorations(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .resizable(false)
        .shadow(false)
        .focused(false)
        .focusable(false)
        .visible(false)
        .build()?;
        w.set_position(PhysicalPosition::new(m.position().x, m.position().y))?;
        w.set_size(PhysicalSize::new(m.size().width, m.size().height))?;
        w.show()?;
    }
    // Watch for you coming back.
    let app = app.clone();
    let shown = Instant::now();
    std::thread::spawn(move || {
        while SHOWING.load(Ordering::SeqCst) {
            std::thread::sleep(Duration::from_millis(250));
            if by_hand && shown.elapsed() < GRACE {
                continue;
            }
            if crate::system::screentime::probe::idle_ms() < BACK_MS {
                hide(&app);
                break;
            }
        }
    });
    Ok(())
}

#[cfg(not(windows))]
pub fn show(_app: &AppHandle, _by_hand: bool) -> AppResult<()> {
    Ok(())
}

/// Bring it up by itself after the idle time in Settings.
pub fn start(app: &AppHandle) {
    #[cfg(windows)]
    {
        let app = app.clone();
        std::thread::Builder::new()
            .name("away-screen".into())
            .spawn(move || loop {
                std::thread::sleep(Duration::from_secs(15));
                let s = app
                    .state::<crate::state::AppState>()
                    .settings
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .away
                    .clone();
                let idle = crate::system::screentime::probe::idle_ms();
                let locked = crate::system::screentime::probe::locked();
                if due(&s, idle, locked, showing()) {
                    if let Err(error) = show(&app, false) {
                        tracing::warn!(kind = error.kind(), "could not show the away screen");
                        SHOWING.store(false, Ordering::SeqCst);
                    }
                } else if locked && showing() {
                    hide(&app);
                }
            })
            .ok();
    }
    #[cfg(not(windows))]
    let _ = app;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn picks_the_other_monitors_unless_all() {
        assert_eq!(targets(1, Some(0), false), vec![0]);
        assert_eq!(targets(3, Some(1), false), vec![0, 2]);
        assert_eq!(targets(2, Some(0), true), vec![0, 1]);
    }

    #[test]
    fn comes_up_after_the_idle_time_unless_off_or_locked() {
        let s = AwaySettings {
            after_idle_min: 5,
            all_screens: false,
        };
        assert!(due(&s, 5 * 60_000, false, false));
        assert!(!due(&s, 4 * 60_000, false, false));
        assert!(
            !due(&s, 9 * 60_000, true, false),
            "locked: Windows' lock screen is up"
        );
        assert!(!due(&s, 9 * 60_000, false, true), "already up");
        assert!(!due(&AwaySettings::default(), 99 * 60_000, false, false), "off");
    }
}
