//! The pet, called out of the notch onto the desktop: a small window you can drag anywhere,
//! where it sits and keeps you company (with a laptop while you code, popcorn while you watch,
//! a speech bubble for its comments). The island decides how it looks and what it says and sends
//! that over; this window only draws it and reports taps and drags.

use tauri::{AppHandle, Manager, PhysicalPosition, WebviewUrl, WebviewWindowBuilder};

use crate::{error::AppResult, state::AppState};

pub const BUDDY_WINDOW: &str = "petbuddy";
/// Logical size: room for the pet, its speech bubble above and the hover buttons.
const WIDTH: f64 = 260.0;
const HEIGHT: f64 = 250.0;

fn settings(app: &AppHandle) -> crate::settings::Settings {
    app.state::<AppState>()
        .settings
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .clone()
}

fn update(app: &AppHandle, change: impl FnOnce(&mut crate::settings::PetSettings)) -> AppResult<()> {
    let state = app.state::<AppState>();
    let mut next = settings(app);
    change(&mut next.pet);
    crate::commands::settings::persist_and_broadcast(&state, next).map(|_| ())
}

/// Where it sits: the saved spot if it's still on a screen, else the bottom-right of the screen
/// the notch is on.
fn place(app: &AppHandle, w: &tauri::WebviewWindow) {
    let pet = settings(app).pet;
    if let (Some(x), Some(y)) = (pet.buddy_x, pet.buddy_y) {
        let on_screen = w.available_monitors().unwrap_or_default().iter().any(|m| {
            let (p, s) = (m.position(), m.size());
            x >= p.x && y >= p.y && x < p.x + s.width as i32 - 40 && y < p.y + s.height as i32 - 40
        });
        if on_screen {
            let _ = w.set_position(PhysicalPosition::new(x, y));
            return;
        }
    }
    let monitor = app
        .get_webview_window(crate::shell::MAIN_WINDOW)
        .and_then(|main| main.current_monitor().ok().flatten())
        .or_else(|| w.primary_monitor().ok().flatten());
    if let Some(m) = monitor {
        let area = m.work_area();
        let scale = m.scale_factor();
        let (bw, bh) = ((WIDTH * scale) as i32, (HEIGHT * scale) as i32);
        let _ = w.set_position(PhysicalPosition::new(
            area.position.x + area.size.width as i32 - bw - (40.0 * scale) as i32,
            area.position.y + area.size.height as i32 - bh,
        ));
    }
}

/// Bring the pet out onto the desktop (and remember it's out).
pub fn show(app: &AppHandle) -> AppResult<()> {
    let existing = app.get_webview_window(BUDDY_WINDOW);
    let created = existing.is_none();
    let w = match existing {
        Some(w) => w,
        None => WebviewWindowBuilder::new(app, BUDDY_WINDOW, WebviewUrl::App("index.html#buddy".into()))
            .title("Overline pet")
            .inner_size(WIDTH, HEIGHT)
            .decorations(false)
            .transparent(true)
            .always_on_top(true)
            .skip_taskbar(true)
            .resizable(false)
            .shadow(false)
            .focused(false)
            .visible(false)
            .build()?,
    };
    // However the window goes (Send back, Alt+F4, a crash of its page), the pet is back in the
    // notch: don't leave the setting saying it's out.
    let handle = app.clone();
    if created {
        w.on_window_event(move |event| {
            if matches!(event, tauri::WindowEvent::Destroyed) && settings(&handle).pet.buddy_out {
                if let Err(error) = update(&handle, |pet| pet.buddy_out = false) {
                    tracing::warn!(kind = error.kind(), "could not save that the pet went back");
                }
            }
        });
    }
    place(app, &w);
    w.show()?;
    if !settings(app).pet.buddy_out {
        update(app, |pet| pet.buddy_out = true)?;
    }
    Ok(())
}

/// Send it back into the notch.
pub fn hide(app: &AppHandle) -> AppResult<()> {
    if let Some(w) = app.get_webview_window(BUDDY_WINDOW) {
        let _ = w.close();
    }
    if settings(app).pet.buddy_out {
        update(app, |pet| pet.buddy_out = false)?;
    }
    Ok(())
}

/// Remember where it was dropped (physical pixels, the window's top-left).
pub fn save_position(app: &AppHandle) -> AppResult<()> {
    let Some(w) = app.get_webview_window(BUDDY_WINDOW) else {
        return Ok(());
    };
    let p = w.outer_position()?;
    update(app, |pet| {
        pet.buddy_x = Some(p.x);
        pet.buddy_y = Some(p.y);
    })
}

/// Hidden over a fullscreen app (a game, a film), back afterwards. Called from the pet loop.
pub fn follow_fullscreen(app: &AppHandle, fullscreen: bool) {
    if !settings(app).pet.buddy_out {
        return;
    }
    if let Some(w) = app.get_webview_window(BUDDY_WINDOW) {
        let visible = w.is_visible().unwrap_or(true);
        if fullscreen && visible {
            let _ = w.hide();
        } else if !fullscreen && !visible {
            let _ = w.show();
        }
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

/// The app in front is a code editor or a terminal.
pub fn coding_in_front() -> bool {
    #[cfg(windows)]
    {
        use windows::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowThreadProcessId};
        // SAFETY: read-only query of the foreground window's process id.
        let pid = unsafe {
            let mut pid = 0;
            GetWindowThreadProcessId(GetForegroundWindow(), Some(&mut pid));
            pid
        };
        let Some(exe) = crate::clipboard::win::exe_of(pid) else {
            return false;
        };
        let stem = std::path::Path::new(&exe)
            .file_stem()
            .map(|s| s.to_string_lossy().to_lowercase())
            .unwrap_or_default();
        crate::developer::is_editor(&stem) || TERMINALS.contains(&stem.as_str())
    }
    #[cfg(not(windows))]
    false
}
