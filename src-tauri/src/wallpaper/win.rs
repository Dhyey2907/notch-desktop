//! The desktop wallpaper through Windows' `IDesktopWallpaper`, per screen.

use windows::{
    core::{HSTRING, PCWSTR, PWSTR},
    Win32::{
        Foundation::POINT,
        Graphics::Gdi::{
            GetMonitorInfoW, MonitorFromPoint, MONITORINFO, MONITORINFOEXW, MONITOR_DEFAULTTONEAREST,
        },
        System::Com::{CoCreateInstance, CoInitializeEx, CoTaskMemFree, CLSCTX_ALL, COINIT_MULTITHREADED},
        UI::Shell::{DesktopWallpaper, IDesktopWallpaper, DWPOS_FILL},
    },
};

use super::{WallpaperScreen, ALL};
use crate::error::{AppError, AppResult};

fn desktop() -> AppResult<IDesktopWallpaper> {
    // SAFETY: plain COM setup on this (blocking-pool) thread; a thread already set up with
    // another model keeps it, which is fine for this out-of-process object.
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
        CoCreateInstance(&DesktopWallpaper, None, CLSCTX_ALL)
            .map_err(|e| AppError::Platform(format!("wallpaper service unavailable: {e}")))
    }
}

/// Take a string COM allocated, then free it.
fn take(p: PWSTR) -> Option<String> {
    if p.is_null() {
        return None;
    }
    // SAFETY: `p` is a valid, NUL-terminated string from COM that we own and free once.
    unsafe {
        let s = p.to_string().ok();
        CoTaskMemFree(Some(p.0 as *const _));
        s
    }
}

/// Windows' display number ("\\.\DISPLAY2" → 2) and whether it's the main screen.
fn display_number(center: POINT) -> (u32, bool) {
    // SAFETY: read-only monitor queries into a struct we own and size correctly.
    unsafe {
        let monitor = MonitorFromPoint(center, MONITOR_DEFAULTTONEAREST);
        let mut info = MONITORINFOEXW::default();
        info.monitorInfo.cbSize = std::mem::size_of::<MONITORINFOEXW>() as u32;
        if !GetMonitorInfoW(monitor, &mut info.monitorInfo as *mut MONITORINFO).as_bool() {
            return (0, false);
        }
        let device = String::from_utf16_lossy(&info.szDevice);
        let digits: String = device
            .trim_end_matches('\0')
            .chars()
            .rev()
            .take_while(|c| c.is_ascii_digit())
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect();
        (digits.parse().unwrap_or(0), info.monitorInfo.dwFlags & 1 == 1)
    }
}

pub fn screens() -> AppResult<Vec<WallpaperScreen>> {
    let dw = desktop()?;
    let mut found = Vec::new();
    // SAFETY: COM calls on a live IDesktopWallpaper; returned strings are freed by `take`.
    unsafe {
        let count = dw
            .GetMonitorDevicePathCount()
            .map_err(|e| AppError::Platform(e.to_string()))?;
        for i in 0..count {
            let Ok(raw) = dw.GetMonitorDevicePathAt(i) else {
                continue;
            };
            let Some(id) = take(raw) else { continue };
            let wide = HSTRING::from(id.as_str());
            // Screens that are known but switched off have no rectangle.
            let Ok(rect) = dw.GetMonitorRECT(&wide) else {
                continue;
            };
            let center = POINT {
                x: (rect.left + rect.right) / 2,
                y: (rect.top + rect.bottom) / 2,
            };
            let (number, primary) = display_number(center);
            let current = dw
                .GetWallpaper(&wide)
                .ok()
                .and_then(take)
                .filter(|p| !p.is_empty());
            found.push((
                number,
                WallpaperScreen {
                    id,
                    label: String::new(),
                    primary,
                    current,
                },
            ));
        }
    }
    found.sort_by_key(|(n, _)| *n);
    Ok(found
        .into_iter()
        .enumerate()
        .map(|(i, (n, mut screen))| {
            screen.label = format!("Screen {}", if n > 0 { n } else { i as u32 + 1 });
            screen
        })
        .collect())
}

pub fn apply(path: &str, target: &str) -> AppResult<()> {
    let dw = desktop()?;
    let picture = HSTRING::from(path);
    // SAFETY: COM calls on a live IDesktopWallpaper with valid wide strings.
    unsafe {
        dw.SetPosition(DWPOS_FILL)
            .map_err(|e| AppError::Platform(e.to_string()))?;
        let result = if target == ALL {
            dw.SetWallpaper(PCWSTR::null(), &picture)
        } else {
            dw.SetWallpaper(&HSTRING::from(target), &picture)
        };
        result.map_err(|e| AppError::Platform(format!("could not set the wallpaper: {e}")))
    }
}
