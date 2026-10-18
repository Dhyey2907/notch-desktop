//! Panic blur: one shortcut covers every screen with a blurred picture of itself, so someone
//! walking past sees nothing readable. The same shortcut, a click or Esc takes it away. The
//! picture is shrunk to a few hundred pixels before it leaves Rust (that's the blur), so no
//! sharp copy of the screen is kept anywhere.

use std::{collections::HashMap, sync::Mutex};

use tauri::{AppHandle, Manager, PhysicalPosition, PhysicalSize, WebviewUrl, WebviewWindowBuilder};

use crate::error::{AppError, AppResult};

/// Window labels start with this ("blur-0", "blur-1", …).
pub const PREFIX: &str = "blur-";
/// The cover picture's width; the page stretches and softens it.
const SMALL_WIDTH: u32 = 96;

/// Cover pictures by window label (data URLs), handed to each cover window once.
static PICTURES: Mutex<Option<HashMap<String, String>>> = Mutex::new(None);

/// The size to shrink a `w`×`h` screen to: `SMALL_WIDTH` wide, same shape.
pub fn small_size(w: u32, h: u32) -> (u32, u32) {
    if w == 0 || h == 0 {
        return (1, 1);
    }
    let sw = SMALL_WIDTH.min(w);
    (sw, ((u64::from(h) * u64::from(sw)) / u64::from(w)).max(1) as u32)
}

fn covers(app: &AppHandle) -> Vec<tauri::WebviewWindow> {
    app.webview_windows()
        .into_iter()
        .filter(|(label, _)| label.starts_with(PREFIX))
        .map(|(_, w)| w)
        .collect()
}

pub fn is_on(app: &AppHandle) -> bool {
    !covers(app).is_empty()
}

/// Cover every screen, or uncover them if they're covered.
pub fn toggle(app: &AppHandle) -> AppResult<()> {
    if is_on(app) {
        off(app);
        return Ok(());
    }
    on(app)
}

pub fn off(app: &AppHandle) {
    for w in covers(app) {
        let _ = w.close();
    }
    *PICTURES.lock().unwrap_or_else(|e| e.into_inner()) = None;
}

#[cfg(windows)]
fn on(app: &AppHandle) -> AppResult<()> {
    use base64::Engine;

    let monitors = app.available_monitors()?;
    let mut pictures = HashMap::new();
    for (i, m) in monitors.iter().enumerate() {
        let (pos, size) = (m.position(), m.size());
        let shot = crate::grab::capture::rect(pos.x, pos.y, size.width as i32, size.height as i32)?;
        let mut rgba = shot.bgra;
        for px in rgba.chunks_exact_mut(4) {
            px.swap(0, 2);
        }
        let image = image::RgbaImage::from_raw(shot.width, shot.height, rgba)
            .ok_or_else(|| AppError::Platform("bad capture".into()))?;
        let (sw, sh) = small_size(shot.width, shot.height);
        let small = image::imageops::resize(&image, sw, sh, image::imageops::FilterType::Triangle);
        let small = image::imageops::blur(&small, 1.5);
        let mut jpeg = Vec::new();
        image::DynamicImage::ImageRgba8(small)
            .to_rgb8()
            .write_to(&mut std::io::Cursor::new(&mut jpeg), image::ImageFormat::Jpeg)
            .map_err(|e| AppError::Platform(e.to_string()))?;
        let label = format!("{PREFIX}{i}");
        pictures.insert(
            label.clone(),
            format!(
                "data:image/jpeg;base64,{}",
                base64::engine::general_purpose::STANDARD.encode(&jpeg)
            ),
        );
        let window = WebviewWindowBuilder::new(app, &label, WebviewUrl::App("index.html#blur".into()))
            .title("Overline privacy cover")
            .decorations(false)
            .always_on_top(true)
            .skip_taskbar(true)
            .resizable(false)
            .shadow(false)
            .visible(false)
            .build()?;
        window.set_position(PhysicalPosition::new(pos.x, pos.y))?;
        window.set_size(PhysicalSize::new(size.width, size.height))?;
        window.show()?;
    }
    *PICTURES.lock().unwrap_or_else(|e| e.into_inner()) = Some(pictures);
    if let Some(first) = covers(app).first() {
        let _ = first.set_focus();
    }
    Ok(())
}

#[cfg(not(windows))]
fn on(_app: &AppHandle) -> AppResult<()> {
    Err(AppError::Platform("not supported".into()))
}

/// The blurred picture for this cover window.
pub fn picture(label: &str) -> Option<String> {
    PICTURES
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .as_ref()
        .and_then(|m| m.get(label).cloned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shrinks_keeping_the_shape() {
        assert_eq!(small_size(1920, 1080), (96, 54));
        assert_eq!(small_size(2560, 1600), (96, 60));
        assert_eq!(small_size(50, 50), (50, 50));
        assert_eq!(small_size(0, 10), (1, 1));
    }
}
