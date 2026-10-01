//! A PNG of one window (the one in front when Ask was opened), rendered by the window itself
//! with `PrintWindow`, so other windows on top don't show. Kept in memory only.

use windows::Win32::{
    Foundation::{HWND, RECT},
    Graphics::Gdi::{
        CreateCompatibleBitmap, CreateCompatibleDC, DeleteDC, DeleteObject, GetDC, GetDIBits, ReleaseDC,
        SelectObject, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS,
    },
    Storage::Xps::{PrintWindow, PRINT_WINDOW_FLAGS},
    UI::WindowsAndMessaging::{GetWindowRect, IsWindow, PW_RENDERFULLCONTENT},
};

/// Screenshots wider than this are scaled down (fewer tokens, still readable).
const MAX_WIDTH: u32 = 1600;

pub fn window_png(hwnd_raw: isize) -> Option<Vec<u8>> {
    let hwnd = HWND(hwnd_raw as *mut _);
    // SAFETY: GDI objects are created, used and released in this block; the pixel buffer is
    // sized from the bitmap header before GetDIBits fills it.
    let (w, h, bgra) = unsafe {
        if !IsWindow(Some(hwnd)).as_bool() {
            return None;
        }
        let mut rect = RECT::default();
        GetWindowRect(hwnd, &mut rect).ok()?;
        let (w, h) = (rect.right - rect.left, rect.bottom - rect.top);
        if w <= 0 || h <= 0 || w > 10_000 || h > 10_000 {
            return None;
        }
        let screen = GetDC(None);
        let dc = CreateCompatibleDC(Some(screen));
        let bitmap = CreateCompatibleBitmap(screen, w, h);
        let old = SelectObject(dc, bitmap.into());
        let printed = PrintWindow(hwnd, dc, PRINT_WINDOW_FLAGS(PW_RENDERFULLCONTENT)).as_bool();
        let mut info = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: w,
                biHeight: -h, // top-down
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };
        let mut pixels = vec![0u8; (w * h * 4) as usize];
        let lines = GetDIBits(
            dc,
            bitmap,
            0,
            h as u32,
            Some(pixels.as_mut_ptr().cast()),
            &mut info,
            DIB_RGB_COLORS,
        );
        SelectObject(dc, old);
        let _ = DeleteObject(bitmap.into());
        let _ = DeleteDC(dc);
        ReleaseDC(None, screen);
        if !printed || lines == 0 {
            return None;
        }
        (w as u32, h as u32, pixels)
    };
    encode(w, h, &bgra)
}

/// BGRA rows -> RGB PNG, scaled down (nearest pixel) to at most `MAX_WIDTH` wide.
fn encode(w: u32, h: u32, bgra: &[u8]) -> Option<Vec<u8>> {
    let scale = (w as f32 / MAX_WIDTH as f32).max(1.0);
    let (ow, oh) = (((w as f32) / scale) as u32, ((h as f32) / scale) as u32);
    let mut rgb = Vec::with_capacity((ow * oh * 3) as usize);
    for y in 0..oh {
        let sy = ((y as f32 * scale) as u32).min(h - 1);
        for x in 0..ow {
            let sx = ((x as f32 * scale) as u32).min(w - 1);
            let i = ((sy * w + sx) * 4) as usize;
            rgb.extend_from_slice(&[bgra[i + 2], bgra[i + 1], bgra[i]]);
        }
    }
    let mut out = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut out, ow, oh);
        encoder.set_color(png::ColorType::Rgb);
        encoder.set_depth(png::BitDepth::Eight);
        encoder.write_header().ok()?.write_image_data(&rgb).ok()?;
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::encode;

    #[test]
    fn scales_wide_screenshots_down() {
        let (w, h) = (3200u32, 10u32);
        let bgra = vec![255u8; (w * h * 4) as usize];
        let png_bytes = encode(w, h, &bgra).unwrap();
        let reader = png::Decoder::new(std::io::Cursor::new(png_bytes))
            .read_info()
            .unwrap();
        assert_eq!((reader.info().width, reader.info().height), (1600, 5));
    }
}
