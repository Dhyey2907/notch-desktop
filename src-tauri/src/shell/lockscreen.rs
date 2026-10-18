//! Lock-screen info: Windows doesn't let apps draw on the lock screen, so Overline draws a small
//! panel (next meeting, weather, what's playing, unread mail) onto a copy of your wallpaper
//! every few minutes and sets that as the lock-screen picture. Turning it off puts your old
//! lock-screen picture back when Windows says what it was; with Windows Spotlight it can't,
//! so Overline opens the lock-screen settings instead.

use std::{
    path::{Path, PathBuf},
    sync::Mutex,
    time::{Duration, Instant},
};

use crate::error::{AppError, AppResult};

/// Updates closer together than this are skipped (every island sends them).
const MIN_GAP: Duration = Duration::from_secs(60);
pub const MAX_LINES: usize = 5;
const MAX_LINE_CHARS: usize = 80;

static LAST: Mutex<Option<(Instant, Vec<String>)>> = Mutex::new(None);

/// Clean the lines: no blanks, short, at most a handful.
pub fn tidy(lines: &[String]) -> Vec<String> {
    lines
        .iter()
        .map(|l| l.trim().replace(['\r', '\n'], " "))
        .filter(|l| !l.is_empty())
        .map(|l| {
            if l.chars().count() > MAX_LINE_CHARS {
                let cut: String = l.chars().take(MAX_LINE_CHARS - 1).collect();
                format!("{cut}…")
            } else {
                l
            }
        })
        .take(MAX_LINES)
        .collect()
}

/// Should this update go through? (Something changed, or it's been a while.)
pub fn due(last: Option<&(Instant, Vec<String>)>, lines: &[String], now: Instant) -> bool {
    let Some((at, prev)) = last else { return true };
    let waited = now.duration_since(*at);
    waited >= MIN_GAP && (prev != lines || waited >= Duration::from_secs(10 * 60))
}

/// The panel's rectangle (left, top, width, height) on a `w`×`h` picture with `n` lines:
/// lower left, clear of the clock (top) and the status icons (bottom right).
pub fn panel_rect(w: u32, h: u32, n: usize) -> (u32, u32, u32, u32) {
    let line = (h / 26).max(18);
    let pad = line / 2;
    let height = pad * 2 + line * n.max(1) as u32;
    let left = w / 20;
    let width = (w * 36 / 100).max(300).min(w - left);
    let top = (h * 64 / 100).min(h.saturating_sub(height + h / 12));
    (left, top, width, height)
}

fn files(dir: &Path) -> (PathBuf, PathBuf, PathBuf) {
    (
        dir.join("lockscreen-a.png"),
        dir.join("lockscreen-b.png"),
        dir.join("lockscreen-original.txt"),
    )
}

#[cfg(windows)]
mod win {
    use std::path::Path;

    use windows::{
        core::HSTRING,
        Storage::StorageFile,
        System::UserProfile::{LockScreen, UserProfilePersonalizationSettings},
        Win32::Foundation::COLORREF,
        Win32::Graphics::Gdi::{
            CreateCompatibleDC, CreateDIBSection, CreateFontW, DeleteDC, DeleteObject, SelectObject,
            SetBkMode, SetTextColor, TextOutW, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, CLEARTYPE_QUALITY,
            DIB_RGB_COLORS, TRANSPARENT,
        },
    };

    use crate::error::{AppError, AppResult};

    /// Draw `lines` in white onto BGRA pixels (top-down), in Segoe UI.
    pub fn draw_text(
        w: u32,
        h: u32,
        bgra: &mut [u8],
        lines: &[String],
        panel: (u32, u32, u32, u32),
    ) -> AppResult<()> {
        let fail = || AppError::Platform("could not draw the lock-screen text".into());
        let line = (h / 26).max(18) as i32;
        // SAFETY: GDI objects created here are released before returning; the DIB's pixels are
        // copied in before drawing and out after, while it is alive.
        unsafe {
            let dc = CreateCompatibleDC(None);
            let header = BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: w as i32,
                biHeight: -(h as i32),
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            };
            let info = BITMAPINFO {
                bmiHeader: header,
                ..Default::default()
            };
            let mut bits = std::ptr::null_mut();
            let dib =
                CreateDIBSection(Some(dc), &info, DIB_RGB_COLORS, &mut bits, None, 0).map_err(|_| fail())?;
            if bits.is_null() {
                let _ = DeleteDC(dc);
                return Err(fail());
            }
            let len = (w * h * 4) as usize;
            std::ptr::copy_nonoverlapping(bgra.as_ptr(), bits.cast::<u8>(), len);
            let old = SelectObject(dc, dib.into());
            let font = CreateFontW(
                -(line * 72 / 100),
                0,
                0,
                0,
                500,
                0,
                0,
                0,
                Default::default(),
                Default::default(),
                Default::default(),
                CLEARTYPE_QUALITY,
                0,
                &HSTRING::from("Segoe UI"),
            );
            let old_font = SelectObject(dc, font.into());
            SetBkMode(dc, TRANSPARENT);
            SetTextColor(dc, COLORREF(0x00FF_FFFF));
            let pad = line / 2;
            for (i, text) in lines.iter().enumerate() {
                let wide: Vec<u16> = text.encode_utf16().collect();
                let _ = TextOutW(
                    dc,
                    panel.0 as i32 + pad,
                    panel.1 as i32 + pad + i as i32 * line,
                    &wide,
                );
            }
            std::ptr::copy_nonoverlapping(bits.cast::<u8>(), bgra.as_mut_ptr(), len);
            SelectObject(dc, old_font);
            let _ = DeleteObject(font.into());
            SelectObject(dc, old);
            let _ = DeleteObject(dib.into());
            let _ = DeleteDC(dc);
        }
        Ok(())
    }

    pub fn set_lock_image(path: &Path) -> AppResult<()> {
        let fail = |e: windows::core::Error| {
            AppError::Platform(format!("Windows wouldn't set the lock screen: {e}"))
        };
        let file = StorageFile::GetFileFromPathAsync(&HSTRING::from(path.as_os_str()))
            .and_then(|op| op.get())
            .map_err(fail)?;
        let settings = UserProfilePersonalizationSettings::Current().map_err(fail)?;
        let ok = settings
            .TrySetLockScreenImageAsync(&file)
            .and_then(|op| op.get())
            .map_err(fail)?;
        if ok {
            Ok(())
        } else {
            Err(AppError::Platform(
                "Windows didn't accept the lock-screen picture".into(),
            ))
        }
    }

    /// The lock screen's picture file now, if it's a file (empty with Windows Spotlight).
    pub fn current_lock_image() -> Option<String> {
        let uri = LockScreen::OriginalImageFile().ok()?;
        let text = uri.AbsoluteUri().ok()?.to_string();
        let path = text.strip_prefix("file:///").map(|p| p.replace('/', "\\"))?;
        (!path.is_empty()).then_some(path)
    }
}

/// Compose the picture: `wallpaper` filling a `size` screen, the panel, the lines.
#[cfg(windows)]
pub fn compose(wallpaper: &Path, size: (u32, u32), lines: &[String]) -> AppResult<Vec<u8>> {
    let (w, h) = size;
    let base = image::open(wallpaper)
        .map_err(|e| AppError::Platform(format!("couldn't open the wallpaper: {e}")))?
        .resize_to_fill(w, h, image::imageops::FilterType::Triangle)
        .to_rgba8();
    let mut bgra = base.into_raw();
    for px in bgra.chunks_exact_mut(4) {
        px.swap(0, 2);
    }
    let panel = panel_rect(w, h, lines.len());
    // Darken the panel so white text reads on any wallpaper.
    for y in panel.1..(panel.1 + panel.3).min(h) {
        for x in panel.0..(panel.0 + panel.2).min(w) {
            let i = ((y * w + x) * 4) as usize;
            for c in 0..3 {
                bgra[i + c] = (u16::from(bgra[i + c]) * 40 / 100) as u8;
            }
        }
    }
    win::draw_text(w, h, &mut bgra, lines, panel)?;
    let mut rgb = Vec::with_capacity((w * h * 3) as usize);
    for px in bgra.chunks_exact(4) {
        rgb.extend_from_slice(&[px[2], px[1], px[0]]);
    }
    let img = image::RgbImage::from_raw(w, h, rgb).ok_or_else(|| AppError::Platform("bad picture".into()))?;
    let mut png = Vec::new();
    img.write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
        .map_err(|e| AppError::Platform(e.to_string()))?;
    Ok(png)
}

/// Draw the lines onto the wallpaper and make it the lock-screen picture (rate-limited).
#[cfg(windows)]
pub fn update(cache: &Path, wallpaper: &Path, size: (u32, u32), lines: &[String]) -> AppResult<bool> {
    let lines = tidy(lines);
    let now = Instant::now();
    {
        let last = LAST.lock().unwrap_or_else(|e| e.into_inner());
        if !due(last.as_ref(), &lines, now) {
            return Ok(false);
        }
    }
    std::fs::create_dir_all(cache)?;
    let (a, b, original) = files(cache);
    // Remember the picture that was there before the first time.
    if !original.exists() {
        let _ = std::fs::write(&original, win::current_lock_image().unwrap_or_default());
    }
    let png = compose(wallpaper, size, &lines)?;
    // Alternate names: Windows keeps showing a cached copy when the same path is set again.
    let target = if a.exists() && !b.exists() { &b } else { &a };
    let other = if target == &a { &b } else { &a };
    std::fs::write(target, png)?;
    win::set_lock_image(target)?;
    let _ = std::fs::remove_file(other);
    *LAST.lock().unwrap_or_else(|e| e.into_inner()) = Some((now, lines));
    Ok(true)
}

/// Turned off: put the old lock-screen picture back. `false` means it was Windows Spotlight or
/// unknown, and the caller should open the lock-screen settings.
#[cfg(windows)]
pub fn restore(cache: &Path) -> bool {
    let (a, b, original) = files(cache);
    *LAST.lock().unwrap_or_else(|e| e.into_inner()) = None;
    let before = std::fs::read_to_string(&original).unwrap_or_default();
    let restored = !before.trim().is_empty()
        && Path::new(before.trim()).is_file()
        && win::set_lock_image(Path::new(before.trim())).is_ok();
    let _ = std::fs::remove_file(&a);
    let _ = std::fs::remove_file(&b);
    let _ = std::fs::remove_file(&original);
    restored
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tidies_lines() {
        let long = "x".repeat(200);
        let lines = tidy(&["  Standup 10:30 ".into(), "".into(), long, "a\nb".into()]);
        assert_eq!(lines[0], "Standup 10:30");
        assert_eq!(lines[1].chars().count(), MAX_LINE_CHARS);
        assert_eq!(lines[2], "a b");
    }

    #[test]
    fn rate_limits_but_lets_changes_through() {
        let t0 = Instant::now();
        let lines = vec!["a".to_string()];
        assert!(due(None, &lines, t0));
        let last = (t0, lines.clone());
        assert!(!due(Some(&last), &lines, t0 + Duration::from_secs(120)));
        assert!(due(Some(&last), &["b".to_string()], t0 + Duration::from_secs(61)));
        assert!(!due(
            Some(&last),
            &["b".to_string()],
            t0 + Duration::from_secs(10)
        ));
        assert!(due(Some(&last), &lines, t0 + Duration::from_secs(11 * 60)));
    }

    #[test]
    fn panel_stays_on_the_picture() {
        for (w, h) in [(1920, 1080), (2560, 1600), (1280, 720)] {
            let (x, y, pw, ph) = panel_rect(w, h, 5);
            assert!(x + pw <= w && y + ph <= h, "{w}x{h}");
            assert!(y > h / 2);
        }
    }
}
