//! The grab flow: capture → a window over that screen showing the frozen picture, where the
//! user draws a box → OCR → clipboard. The picture stays in memory only.

use std::sync::{Arc, Mutex};

use tauri::{AppHandle, Manager, PhysicalPosition, PhysicalSize, WebviewUrl, WebviewWindowBuilder};

use super::Shot;
use crate::{
    error::{AppError, AppResult},
    events::AppEvent,
    state::AppState,
};

pub const GRAB_WINDOW: &str = "grab";
/// Crops shorter than this are enlarged before reading (small text reads better bigger).
const SMALL: u32 = 60;

#[derive(Clone)]
pub struct GrabService {
    app: AppHandle,
    shot: Arc<Mutex<Option<Shot>>>,
}

impl GrabService {
    pub fn new(app: AppHandle) -> Self {
        Self {
            app,
            shot: Arc::default(),
        }
    }

    /// Freeze the screen under the pointer and open the selection window over it.
    pub async fn begin(&self) -> AppResult<()> {
        #[cfg(windows)]
        {
            let shot = tauri::async_runtime::spawn_blocking(super::capture::screen_under_pointer)
                .await
                .map_err(|e| AppError::Platform(e.to_string()))??;
            let (x, y, w, h) = (shot.x, shot.y, shot.width, shot.height);
            *self.shot.lock().unwrap_or_else(|e| e.into_inner()) = Some(shot);
            let window = match self.app.get_webview_window(GRAB_WINDOW) {
                Some(window) => window,
                None => WebviewWindowBuilder::new(
                    &self.app,
                    GRAB_WINDOW,
                    WebviewUrl::App("index.html#grab".into()),
                )
                .title("Grab text")
                .decorations(false)
                .always_on_top(true)
                .skip_taskbar(true)
                .resizable(false)
                .shadow(false)
                .visible(false)
                .build()?,
            };
            window.set_position(PhysicalPosition::new(x, y))?;
            window.set_size(PhysicalSize::new(w, h))?;
            // Tell an already-open window to load the new picture.
            window.eval("window.dispatchEvent(new Event('notch-grab'))")?;
            window.show()?;
            window.set_focus()?;
            Ok(())
        }
        #[cfg(not(windows))]
        Err(AppError::Platform("not supported".into()))
    }

    /// The frozen screen as a JPEG data URL, for the selection window to show.
    pub fn picture(&self) -> AppResult<String> {
        use base64::{engine::general_purpose::STANDARD, Engine};
        let shot = self
            .shot
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
            .ok_or_else(|| AppError::InvalidInput("nothing captured".into()))?;
        let rgb: Vec<u8> = shot
            .bgra
            .chunks_exact(4)
            .flat_map(|p| [p[2], p[1], p[0]])
            .collect();
        let mut jpeg = Vec::new();
        image::codecs::jpeg::JpegEncoder::new_with_quality(&mut jpeg, 88)
            .encode(&rgb, shot.width, shot.height, image::ExtendedColorType::Rgb8)
            .map_err(|e| AppError::Platform(e.to_string()))?;
        Ok(format!("data:image/jpeg;base64,{}", STANDARD.encode(jpeg)))
    }

    fn close(&self) {
        *self.shot.lock().unwrap_or_else(|e| e.into_inner()) = None;
        if let Some(window) = self.app.get_webview_window(GRAB_WINDOW) {
            let _ = window.hide();
        }
    }

    pub fn cancel(&self) {
        self.close();
    }

    /// Read the text in the box (screen pixels) and copy it. Returns the text.
    pub fn finish(&self, x: u32, y: u32, w: u32, h: u32) -> AppResult<String> {
        let shot = self.shot.lock().unwrap_or_else(|e| e.into_inner()).clone();
        self.close();
        let shot = shot.ok_or_else(|| AppError::InvalidInput("nothing captured".into()))?;
        let (cw, ch, pixels) = super::crop(&shot, x, y, w, h)
            .ok_or_else(|| AppError::InvalidInput("Draw a bigger box".into()))?;
        let (cw, ch, pixels) = if ch < SMALL {
            super::upscale2(cw, ch, &pixels)
        } else {
            (cw, ch, pixels)
        };
        #[cfg(windows)]
        let text = super::ocr::read(cw, ch, &pixels)?;
        #[cfg(not(windows))]
        let text = {
            let _ = (cw, ch, pixels);
            String::new()
        };
        #[cfg(windows)]
        if !text.trim().is_empty() {
            crate::clipboard::win::write(crate::clipboard::win::Put::Text(&text))?;
        }
        self.app
            .state::<AppState>()
            .bus
            .emit("grab", AppEvent::TextGrabbed { text: text.clone() });
        Ok(text)
    }
}
