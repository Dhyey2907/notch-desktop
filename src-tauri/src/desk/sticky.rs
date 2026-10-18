//! Sticky notes on the desktop: a note from Notes pinned as its own small window that you can
//! move, resize, recolour and edit. They stay where you put them (and come back after a
//! restart) but aren't always on top: they live with your other windows.

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, PhysicalPosition, PhysicalSize, WebviewUrl, WebviewWindowBuilder};
use ts_rs::TS;

use crate::error::{AppError, AppResult};

pub const PREFIX: &str = "sticky-";
pub const MAX_STICKIES: usize = 12;
pub const COLORS: &[&str] = &["yellow", "pink", "blue", "green", "gray"];

/// One sticky: which note, where, how big, which colour.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Sticky {
    pub note_id: String,
    /// Physical pixels on the desktop.
    pub x: i32,
    pub y: i32,
    pub w: u32,
    pub h: u32,
    pub color: String,
}

/// Note ids are UUIDs; anything else can't become a window label.
pub fn valid_id(id: &str) -> bool {
    !id.is_empty() && id.len() <= 64 && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
}

pub fn label(note_id: &str) -> String {
    format!("{PREFIX}{note_id}")
}

pub fn valid(s: &Sticky) -> bool {
    valid_id(&s.note_id)
        && (120..=1600).contains(&s.w)
        && (100..=1600).contains(&s.h)
        && COLORS.contains(&s.color.as_str())
        && s.x.abs() < 40_000
        && s.y.abs() < 40_000
}

/// Open (or bring forward) the window for one sticky.
pub fn show(app: &AppHandle, s: &Sticky) -> AppResult<()> {
    if !valid(s) {
        return Err(AppError::InvalidInput("invalid sticky".into()));
    }
    let label = label(&s.note_id);
    if let Some(w) = app.get_webview_window(&label) {
        w.show()?;
        w.set_focus()?;
        return Ok(());
    }
    let w = WebviewWindowBuilder::new(
        app,
        &label,
        WebviewUrl::App(format!("index.html#sticky={}", s.note_id).into()),
    )
    .title("Sticky note")
    .decorations(false)
    .skip_taskbar(true)
    .resizable(true)
    .shadow(true)
    .min_inner_size(140.0, 110.0)
    .visible(false)
    .build()?;
    w.set_size(PhysicalSize::new(s.w, s.h))?;
    w.set_position(PhysicalPosition::new(s.x, s.y))?;
    w.show()?;
    Ok(())
}

pub fn close(app: &AppHandle, note_id: &str) {
    if let Some(w) = app.get_webview_window(&label(note_id)) {
        let _ = w.close();
    }
}

/// A new sticky's place: near the top right of the main screen, each a little lower.
pub fn place_new(screen: (i32, i32, u32, u32), scale: f64, index: usize) -> (i32, i32, u32, u32) {
    let w = (260.0 * scale) as u32;
    let h = (220.0 * scale) as u32;
    let step = (28.0 * scale) as i32 * (index as i32 % 8);
    let x = screen.0 + screen.2 as i32 - w as i32 - (40.0 * scale) as i32 - step;
    let y = screen.1 + (80.0 * scale) as i32 + step;
    (x, y, w, h)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checks_ids_and_sizes() {
        assert!(valid_id("2f9c1d1e-aaaa-bbbb-cccc-1234567890ab"));
        assert!(!valid_id("../evil"));
        let s = Sticky {
            note_id: "abc".into(),
            x: 10,
            y: 10,
            w: 260,
            h: 220,
            color: "yellow".into(),
        };
        assert!(valid(&s));
        assert!(!valid(&Sticky {
            color: "red;".into(),
            ..s.clone()
        }));
        assert!(!valid(&Sticky { w: 50, ..s }));
    }

    #[test]
    fn new_stickies_step_down_from_the_top_right() {
        let a = place_new((0, 0, 1920, 1040), 1.0, 0);
        let b = place_new((0, 0, 1920, 1040), 1.0, 1);
        assert_eq!(a, (1620, 80, 260, 220));
        assert_eq!((b.0, b.1), (1592, 108));
    }
}
