//! Picture-in-picture: any window (a video in the browser, a stream, a call) shown live in a
//! small always-on-top window you can move and resize, cropped to the part you care about.
//! Windows draws it (DWM); nothing is captured or recorded.

use std::sync::Mutex;

use serde::Serialize;
use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};
use ts_rs::TS;

use crate::error::{AppError, AppResult};

pub const PIP_WINDOW: &str = "pip";

/// What the picture-in-picture window shows.
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct PipSource {
    #[ts(type = "number")]
    pub id: i64,
    pub app: String,
    pub title: String,
}

struct State {
    source: Option<PipSource>,
    thumb: Option<isize>,
}

static STATE: Mutex<State> = Mutex::new(State {
    source: None,
    thumb: None,
});

/// Open (or retarget) the picture-in-picture window on window `id`.
pub fn open(app: &AppHandle, id: i64) -> AppResult<()> {
    #[cfg(windows)]
    let source = super::win::list()
        .into_iter()
        .find(|w| w.id == id)
        .map(|w| PipSource {
            id: w.id,
            app: w.app,
            title: w.title,
        })
        .ok_or_else(|| AppError::InvalidInput("That window was closed".into()))?;
    #[cfg(not(windows))]
    let source = PipSource {
        id,
        app: String::new(),
        title: String::new(),
    };
    {
        let mut st = STATE.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(thumb) = st.thumb.take() {
            #[cfg(windows)]
            super::win::drop_thumbnail(thumb);
            #[cfg(not(windows))]
            let _ = thumb;
        }
        st.source = Some(source);
    }
    match app.get_webview_window(PIP_WINDOW) {
        Some(w) => {
            // Tell the open window to show the new source.
            w.eval("window.dispatchEvent(new Event('overline-pip'))")?;
            w.show()?;
        }
        None => {
            WebviewWindowBuilder::new(app, PIP_WINDOW, WebviewUrl::App("index.html#pip".into()))
                .title("Overline picture-in-picture")
                .inner_size(480.0, 300.0)
                .min_inner_size(200.0, 130.0)
                .decorations(false)
                .always_on_top(true)
                .skip_taskbar(true)
                .resizable(true)
                .shadow(true)
                .build()?;
        }
    }
    Ok(())
}

pub fn source() -> Option<PipSource> {
    STATE.lock().unwrap_or_else(|e| e.into_inner()).source.clone()
}

/// Draw the source into the picture-in-picture window at `area` (physical pixels), cropped to
/// `crop` (fractions of the source). Returns the source window's size.
pub fn place(
    app: &AppHandle,
    area: (i32, i32, i32, i32),
    crop: (f64, f64, f64, f64),
    dim: bool,
) -> AppResult<(i32, i32)> {
    #[cfg(windows)]
    {
        let window = app
            .get_webview_window(PIP_WINDOW)
            .ok_or_else(|| AppError::InvalidInput("no picture-in-picture window".into()))?;
        let mut st = STATE.lock().unwrap_or_else(|e| e.into_inner());
        let source = st
            .source
            .clone()
            .ok_or_else(|| AppError::InvalidInput("nothing to show".into()))?;
        let thumb = match st.thumb {
            Some(t) => t,
            None => {
                let dest = windows::Win32::Foundation::HWND(window.hwnd()?.0);
                let t = super::win::thumbnail(dest, source.id, area)?;
                st.thumb = Some(t);
                t
            }
        };
        // Dimmed while cropping, so the box drawn underneath shows through.
        super::win::crop_thumbnail(thumb, area, crop, if dim { 90 } else { 255 })
    }
    #[cfg(not(windows))]
    {
        let _ = (app, area, crop, dim);
        Ok((0, 0))
    }
}

pub fn close(app: &AppHandle) {
    let mut st = STATE.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(thumb) = st.thumb.take() {
        #[cfg(windows)]
        super::win::drop_thumbnail(thumb);
        #[cfg(not(windows))]
        let _ = thumb;
    }
    st.source = None;
    drop(st);
    if let Some(w) = app.get_webview_window(PIP_WINDOW) {
        let _ = w.close();
    }
}

#[cfg(all(test, windows))]
mod tests {
    use super::super::win::crop_rect;

    #[test]
    fn crops_by_fractions_and_never_to_nothing() {
        assert_eq!(crop_rect((1000, 500), (0.1, 0.2, 0.5, 0.5)), (100, 100, 600, 350));
        assert_eq!(crop_rect((1000, 500), (0.0, 0.0, 1.0, 1.0)), (0, 0, 1000, 500));
        let tiny = crop_rect((1000, 500), (0.5, 0.5, 0.0, 0.0));
        assert!(tiny.2 > tiny.0 && tiny.3 > tiny.1);
    }
}
