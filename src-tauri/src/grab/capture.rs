//! Capture the screen the pointer is on (GDI), as BGRA pixels.

use windows::Win32::{
    Foundation::POINT,
    Graphics::Gdi::{
        BitBlt, CreateCompatibleDC, CreateDIBSection, DeleteDC, DeleteObject, GetDC, GetMonitorInfoW,
        MonitorFromPoint, ReleaseDC, SelectObject, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, CAPTUREBLT,
        DIB_RGB_COLORS, MONITORINFO, MONITOR_DEFAULTTONEAREST, SRCCOPY,
    },
    UI::WindowsAndMessaging::GetCursorPos,
};

use super::Shot;
use crate::error::{AppError, AppResult};

pub fn screen_under_pointer() -> AppResult<Shot> {
    let fail = || AppError::Platform("could not capture the screen".into());
    // SAFETY: GDI objects created here are released before returning; the DIB's pixels are
    // copied out while it is alive.
    unsafe {
        let mut cursor = POINT::default();
        GetCursorPos(&mut cursor).map_err(|_| fail())?;
        let monitor = MonitorFromPoint(cursor, MONITOR_DEFAULTTONEAREST);
        let mut info = MONITORINFO {
            cbSize: std::mem::size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        if !GetMonitorInfoW(monitor, &mut info).as_bool() {
            return Err(fail());
        }
        let r = info.rcMonitor;
        let (w, h) = (r.right - r.left, r.bottom - r.top);
        if w <= 0 || h <= 0 {
            return Err(fail());
        }
        let screen = GetDC(None);
        let memory = CreateCompatibleDC(Some(screen));
        let header = BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: w,
            biHeight: -h, // top-down
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB.0,
            ..Default::default()
        };
        let bitmap_info = BITMAPINFO {
            bmiHeader: header,
            ..Default::default()
        };
        let mut bits = std::ptr::null_mut();
        let result = CreateDIBSection(Some(memory), &bitmap_info, DIB_RGB_COLORS, &mut bits, None, 0)
            .ok()
            .and_then(|dib| {
                let old = SelectObject(memory, dib.into());
                let copied = BitBlt(
                    memory,
                    0,
                    0,
                    w,
                    h,
                    Some(screen),
                    r.left,
                    r.top,
                    SRCCOPY | CAPTUREBLT,
                )
                .is_ok();
                let pixels = (copied && !bits.is_null())
                    .then(|| std::slice::from_raw_parts(bits.cast::<u8>(), (w * h * 4) as usize).to_vec());
                SelectObject(memory, old);
                let _ = DeleteObject(dib.into());
                pixels
            });
        let _ = DeleteDC(memory);
        ReleaseDC(None, screen);
        let mut bgra = result.ok_or_else(fail)?;
        // GDI leaves alpha at 0; make it opaque.
        for px in bgra.chunks_exact_mut(4) {
            px[3] = 255;
        }
        Ok(Shot {
            width: w as u32,
            height: h as u32,
            bgra,
            x: r.left,
            y: r.top,
        })
    }
}
