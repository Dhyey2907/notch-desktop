//! Desk: your open windows (pin on top, see-through, a live mini view in the notch) and
//! routines — one click (or a schedule) opens apps, sites and folders, closes clutter and can
//! start a focus session.

pub mod pip;
pub mod place;
pub mod service;
pub mod sticky;
#[cfg(windows)]
pub(crate) mod win;
pub mod workspace;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// A top-level window someone could want to pin or watch.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct DeskWindow {
    /// The window handle, as a number (only meaningful to this PC, right now).
    #[ts(type = "number")]
    pub id: i64,
    pub title: String,
    /// "Chrome", "Visual Studio Code".
    pub app: String,
    pub on_top: bool,
    /// 20–100 (100 = solid).
    pub opacity: u8,
}

/// One thing a routine does.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", tag = "kind")]
#[ts(export)]
pub enum RoutineStep {
    /// Start a program or open a shortcut (.exe / .lnk).
    App { path: String },
    /// Open a website.
    Url { url: String },
    /// Open a folder in File Explorer.
    Folder { path: String },
    /// Close an app's windows (politely, like clicking ×), by program name.
    Close { exe: String },
    /// Start a focus session.
    Focus { minutes: u32 },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Routine {
    pub id: String,
    pub name: String,
    pub steps: Vec<RoutineStep>,
    /// Run by itself at this time (minutes after midnight) on these days (1 = Monday).
    #[ts(optional)]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub at_min: Option<u32>,
    #[serde(default)]
    pub days: Vec<u8>,
}

pub const MAX_ROUTINES: usize = 20;
pub const MAX_STEPS: usize = 20;

/// Check a routine before saving it.
pub fn validate(r: &Routine) -> Result<(), &'static str> {
    if r.name.trim().is_empty() || r.name.len() > 40 {
        return Err("Give the routine a short name");
    }
    if r.steps.is_empty() || r.steps.len() > MAX_STEPS {
        return Err("A routine has 1 to 20 steps");
    }
    if r.at_min.is_some_and(|m| m >= 24 * 60) || r.days.iter().any(|d| !(1..=7).contains(d)) {
        return Err("Invalid schedule");
    }
    for step in &r.steps {
        let ok = match step {
            RoutineStep::App { path } | RoutineStep::Folder { path } => {
                !path.trim().is_empty() && path.len() <= 1000 && !path.contains('"')
            }
            RoutineStep::Url { url } => {
                (url.starts_with("https://") || url.starts_with("http://")) && url.len() <= 2000
            }
            RoutineStep::Close { exe } => {
                exe.to_lowercase().ends_with(".exe")
                    && exe.len() <= 100
                    && !exe.contains(['\\', '/'])
                    && !PROTECTED.contains(&exe.to_lowercase().as_str())
            }
            RoutineStep::Focus { minutes } => (5..=240).contains(minutes),
        };
        if !ok {
            return Err("One of the steps isn't valid");
        }
    }
    Ok(())
}

/// Never closed by a routine: closing these would break Windows or Notch itself.
const PROTECTED: &[&str] = &[
    "explorer.exe",
    "notch-desktop.exe",
    "dwm.exe",
    "winlogon.exe",
    "csrss.exe",
];

/// Whether a scheduled routine should run now (once per day, within a minute of its time).
pub fn due(r: &Routine, weekday: u8, minute_of_day: u32, already_ran_today: bool) -> bool {
    match r.at_min {
        Some(at) => {
            !already_ran_today
                && (r.days.is_empty() || r.days.contains(&weekday))
                && minute_of_day >= at
                && minute_of_day < at + 2
        }
        None => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn routine(steps: Vec<RoutineStep>) -> Routine {
        Routine {
            id: "r1".into(),
            name: "Start work".into(),
            steps,
            at_min: Some(9 * 60),
            days: vec![1, 2, 3, 4, 5],
        }
    }

    #[test]
    fn checks_steps() {
        assert!(validate(&routine(vec![
            RoutineStep::App {
                path: r"C:\Program Files\App\app.exe".into()
            },
            RoutineStep::Url {
                url: "https://mail.google.com".into()
            },
            RoutineStep::Close {
                exe: "Discord.exe".into()
            },
            RoutineStep::Focus { minutes: 50 },
        ]))
        .is_ok());
        assert!(validate(&routine(vec![])).is_err());
        assert!(validate(&routine(vec![RoutineStep::Url {
            url: "javascript:alert(1)".into()
        }]))
        .is_err());
        assert!(validate(&routine(vec![RoutineStep::Close {
            exe: "explorer.exe".into()
        }]))
        .is_err());
        assert!(validate(&routine(vec![RoutineStep::Close {
            exe: r"C:\x\app.exe".into()
        }]))
        .is_err());
        assert!(validate(&routine(vec![RoutineStep::Focus { minutes: 1 }])).is_err());
    }

    #[cfg(windows)]
    #[test]
    fn fits_the_window_shape() {
        // A wide window in a squarer area: full width, centered vertically.
        assert_eq!(win::fit((0, 0, 400, 300), (1600, 900)), (0, 37, 400, 262));
        // A tall window: full height, centered horizontally.
        assert_eq!(win::fit((10, 10, 410, 310), (500, 1000)), (135, 10, 285, 310));
        assert_eq!(win::fit((0, 0, 100, 100), (0, 0)), (0, 0, 100, 100));
    }

    #[test]
    fn runs_on_schedule_once() {
        let r = routine(vec![RoutineStep::Focus { minutes: 25 }]);
        assert!(due(&r, 1, 540, false));
        assert!(due(&r, 1, 541, false));
        assert!(!due(&r, 1, 542, false));
        assert!(!due(&r, 1, 540, true));
        assert!(!due(&r, 6, 540, false)); // Saturday
        let manual = Routine { at_min: None, ..r };
        assert!(!due(&manual, 1, 540, false));
    }
}
