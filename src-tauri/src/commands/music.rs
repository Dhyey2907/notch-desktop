use tauri::{AppHandle, Manager, State, WebviewUrl, WebviewWindowBuilder};

use crate::music::{service::MusicService, Lyrics, Palette};

/// Synced lyrics for the song playing now, if found.
#[tauri::command]
pub fn lyrics_current(music: State<'_, MusicService>) -> Option<Lyrics> {
    music.lyrics()
}

/// Two colors from a cached album-art file, for tinting the notch.
#[tauri::command]
pub async fn artwork_palette(path: String, music: State<'_, MusicService>) -> Result<Option<Palette>, ()> {
    let music = music.inner().clone();
    Ok(tauri::async_runtime::spawn_blocking(move || music.palette(&path))
        .await
        .ok()
        .flatten())
}

/// The floating lyrics tile's window.
pub const LYRICS_WINDOW: &str = "lyrics";

/// Where the lyrics tile was last left (physical pixels), saved next to the database.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
struct TileRect {
    x: i32,
    y: i32,
    w: u32,
    h: u32,
}

impl TileRect {
    fn sane(self) -> bool {
        (200..=4000).contains(&self.w)
            && (200..=4000).contains(&self.h)
            && self.x.abs() < 40_000
            && self.y.abs() < 40_000
    }
}

fn tile_file(app: &AppHandle) -> std::path::PathBuf {
    app.state::<crate::state::AppState>()
        .paths
        .data_dir
        .join("lyrics-tile.json")
}

/// Bumped on every move or resize; a save only goes ahead if nothing moved since.
static TILE_MOVES: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// Save the tile's spot half a second after it stops moving (a drag sends many moves).
fn remember_tile(app: &AppHandle) {
    use std::sync::atomic::Ordering;
    let ticket = TILE_MOVES.fetch_add(1, Ordering::SeqCst) + 1;
    let app = app.clone();
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(500));
        if TILE_MOVES.load(Ordering::SeqCst) != ticket {
            return;
        }
        let Some(w) = app.get_webview_window(LYRICS_WINDOW) else {
            return;
        };
        let (Ok(pos), Ok(size)) = (w.outer_position(), w.outer_size()) else {
            return;
        };
        let rect = TileRect {
            x: pos.x,
            y: pos.y,
            w: size.width,
            h: size.height,
        };
        if rect.sane() {
            if let Ok(text) = serde_json::to_string(&rect) {
                let _ = std::fs::write(tile_file(&app), text);
            }
        }
    });
}

/// Open the floating lyrics tile (or bring it forward): the whole song, following along, over
/// the album's colors. Frameless, moved by its top bar, resizable, kept on top, never takes
/// the keyboard, and back where you left it. Async: building a window from a synchronous command deadlocks on Windows.
#[tauri::command]
pub async fn lyrics_window_open(app: AppHandle) -> crate::error::AppResult<()> {
    if let Some(w) = app.get_webview_window(LYRICS_WINDOW) {
        w.unminimize()?;
        w.show()?;
        return Ok(());
    }
    let w = WebviewWindowBuilder::new(&app, LYRICS_WINDOW, WebviewUrl::App("index.html#lyrics".into()))
        .title("Lyrics")
        .inner_size(340.0, 460.0)
        .min_inner_size(240.0, 260.0)
        .decorations(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .resizable(true)
        .shadow(true)
        .center()
        // A passive overlay: it never takes the keyboard from the app you're typing in, but
        // still scrolls, takes clicks and moves by its bar.
        .focused(false)
        .focusable(false)
        .visible(false)
        .build()?;
    let saved = std::fs::read_to_string(tile_file(&app))
        .ok()
        .and_then(|t| serde_json::from_str::<TileRect>(&t).ok())
        .filter(|r| r.sane());
    // Only on a screen that's still there.
    let on_screen = saved.filter(|r| {
        w.available_monitors().is_ok_and(|ms| {
            ms.iter().any(|m| {
                let (p, s) = (m.position(), m.size());
                r.x + 40 >= p.x
                    && r.x + 40 < p.x + s.width as i32
                    && r.y >= p.y
                    && r.y < p.y + s.height as i32
            })
        })
    });
    if let Some(r) = on_screen {
        // Onto its screen first, then sized: sized elsewhere, Windows would rescale it.
        w.set_position(tauri::PhysicalPosition::new(r.x, r.y))?;
        w.set_size(tauri::PhysicalSize::new(r.w, r.h))?;
    }
    let handle = app.clone();
    w.on_window_event(move |event| {
        if matches!(
            event,
            tauri::WindowEvent::Moved(_) | tauri::WindowEvent::Resized(_)
        ) {
            remember_tile(&handle);
        }
    });
    w.show()?;
    Ok(())
}

/// Close the lyrics tile (its spot is already saved).
#[tauri::command]
pub async fn lyrics_window_close(app: AppHandle) {
    if let Some(w) = app.get_webview_window(LYRICS_WINDOW) {
        let _ = w.close();
    }
}

#[cfg(test)]
mod tests {
    use super::TileRect;

    #[test]
    fn a_saved_tile_spot_must_be_a_sensible_size_and_place() {
        let ok = TileRect {
            x: -1600,
            y: 120,
            w: 340,
            h: 460,
        };
        assert!(ok.sane());
        assert!(!TileRect { w: 10, ..ok }.sane());
        assert!(!TileRect { x: 90_000, ..ok }.sane());
    }
}
