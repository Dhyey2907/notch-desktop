//! Screen recording: the screen under the pointer, cursor included, captured with GDI and
//! encoded by Windows' own H.264 encoder (Media Foundation) to an MP4, or kept small and
//! written as an animated GIF for short clips. Nothing leaves the PC.

use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread::JoinHandle,
    time::Duration,
};

use crate::error::{AppError, AppResult};

/// MP4 frames per second.
pub const MP4_FPS: u32 = 20;
/// GIF frames per second, width cap and length cap (GIFs get big fast).
pub const GIF_FPS: u32 = 10;
pub const GIF_MAX_WIDTH: u32 = 800;
pub const GIF_MAX_LENGTH: Duration = Duration::from_secs(30);
/// MP4s stop on their own after this.
pub const MP4_MAX_LENGTH: Duration = Duration::from_secs(60 * 60);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Mp4,
    Gif,
}

/// H.264 wants even sizes.
pub fn even(w: i32, h: i32) -> (i32, i32) {
    (w & !1, h & !1)
}

/// A bitrate that keeps text sharp without huge files (bits per second).
pub fn bitrate(w: i32, h: i32, fps: u32) -> u32 {
    let pixels = f64::from(w.max(1)) * f64::from(h.max(1)) * f64::from(fps);
    (pixels * 0.15).clamp(1_500_000.0, 25_000_000.0) as u32
}

/// The GIF's size: at most `GIF_MAX_WIDTH` wide, same shape.
pub fn gif_size(w: u32, h: u32) -> (u32, u32) {
    if w <= GIF_MAX_WIDTH || w == 0 {
        return (w.max(1), h.max(1));
    }
    (
        GIF_MAX_WIDTH,
        (u64::from(h) * u64::from(GIF_MAX_WIDTH) / u64::from(w)).max(1) as u32,
    )
}

pub struct ScreenRecording {
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<AppResult<PathBuf>>>,
    pub kind: Kind,
}

impl ScreenRecording {
    /// Stop and wait for the file to be finished.
    pub fn finish(mut self) -> AppResult<PathBuf> {
        self.stop.store(true, Ordering::SeqCst);
        self.thread
            .take()
            .ok_or_else(|| AppError::Platform("recording already finished".into()))?
            .join()
            .map_err(|_| AppError::Platform("the recorder crashed".into()))?
    }
}

impl Drop for ScreenRecording {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
    }
}

/// Start recording the screen at `area` (left, top, width, height in physical pixels) to `out`.
#[cfg(windows)]
pub fn start(out: PathBuf, kind: Kind, area: (i32, i32, i32, i32)) -> ScreenRecording {
    let stop = Arc::new(AtomicBool::new(false));
    let flag = stop.clone();
    let thread = std::thread::Builder::new()
        .name("screen-recorder".into())
        .spawn(move || match kind {
            Kind::Mp4 => win::record_mp4(&out, area, &flag).map(|()| out),
            Kind::Gif => win::record_gif(&out, area, &flag).map(|()| out),
        })
        .ok();
    ScreenRecording { stop, thread, kind }
}

#[cfg(windows)]
mod win {
    use std::{
        path::Path,
        sync::atomic::{AtomicBool, Ordering},
        time::{Duration, Instant},
    };

    use windows::{
        core::HSTRING,
        Win32::{
            Graphics::Gdi::{
                BitBlt, CreateCompatibleDC, CreateDIBSection, DeleteDC, DeleteObject, GetDC, ReleaseDC,
                SelectObject, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, CAPTUREBLT, DIB_RGB_COLORS, HDC, SRCCOPY,
            },
            Media::MediaFoundation::{
                IMFSinkWriter, MFCreateMediaType, MFCreateMemoryBuffer, MFCreateSample,
                MFCreateSinkWriterFromURL, MFMediaType_Video, MFShutdown, MFStartup, MFVideoFormat_H264,
                MFVideoFormat_RGB32, MFVideoInterlace_Progressive, MFSTARTUP_FULL, MF_MT_AVG_BITRATE,
                MF_MT_DEFAULT_STRIDE, MF_MT_FRAME_RATE, MF_MT_FRAME_SIZE, MF_MT_INTERLACE_MODE,
                MF_MT_MAJOR_TYPE, MF_MT_PIXEL_ASPECT_RATIO, MF_MT_SUBTYPE, MF_VERSION,
            },
            UI::WindowsAndMessaging::{
                DrawIconEx, GetCursorInfo, GetIconInfo, CURSORINFO, CURSOR_SHOWING, DI_NORMAL, ICONINFO,
            },
        },
    };

    use super::{bitrate, even, gif_size, GIF_FPS, GIF_MAX_LENGTH, MP4_FPS, MP4_MAX_LENGTH};
    use crate::error::{AppError, AppResult};

    /// One reusable capture surface: a DIB the screen is copied into each frame.
    struct Grabber {
        screen: HDC,
        memory: HDC,
        dib: windows::Win32::Graphics::Gdi::HBITMAP,
        old: windows::Win32::Graphics::Gdi::HGDIOBJ,
        bits: *mut core::ffi::c_void,
        area: (i32, i32, i32, i32),
    }

    impl Grabber {
        fn new(area: (i32, i32, i32, i32)) -> AppResult<Self> {
            let fail = || AppError::Platform("could not capture the screen".into());
            // SAFETY: GDI objects created here are released in Drop.
            unsafe {
                let screen = GetDC(None);
                let memory = CreateCompatibleDC(Some(screen));
                let header = BITMAPINFOHEADER {
                    biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                    biWidth: area.2,
                    biHeight: -area.3,
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
                let dib = CreateDIBSection(Some(memory), &info, DIB_RGB_COLORS, &mut bits, None, 0)
                    .map_err(|_| fail())?;
                if bits.is_null() {
                    return Err(fail());
                }
                let old = SelectObject(memory, dib.into());
                Ok(Self {
                    screen,
                    memory,
                    dib,
                    old,
                    bits,
                    area,
                })
            }
        }

        /// Copy the screen (and the pointer on it) into the surface; return its BGRA pixels.
        fn grab(&self) -> &[u8] {
            let (x, y, w, h) = self.area;
            // SAFETY: drawing into our own DIB; the slice lives as long as `self`.
            unsafe {
                let _ = BitBlt(
                    self.memory,
                    0,
                    0,
                    w,
                    h,
                    Some(self.screen),
                    x,
                    y,
                    SRCCOPY | CAPTUREBLT,
                );
                let mut ci = CURSORINFO {
                    cbSize: std::mem::size_of::<CURSORINFO>() as u32,
                    ..Default::default()
                };
                if GetCursorInfo(&mut ci).is_ok() && ci.flags.0 & CURSOR_SHOWING.0 != 0 {
                    let mut ii = ICONINFO::default();
                    let (hx, hy) = if GetIconInfo(ci.hCursor.into(), &mut ii).is_ok() {
                        let _ = DeleteObject(ii.hbmMask.into());
                        if !ii.hbmColor.is_invalid() {
                            let _ = DeleteObject(ii.hbmColor.into());
                        }
                        (ii.xHotspot as i32, ii.yHotspot as i32)
                    } else {
                        (0, 0)
                    };
                    let _ = DrawIconEx(
                        self.memory,
                        ci.ptScreenPos.x - x - hx,
                        ci.ptScreenPos.y - y - hy,
                        ci.hCursor.into(),
                        0,
                        0,
                        0,
                        None,
                        DI_NORMAL,
                    );
                }
                std::slice::from_raw_parts(self.bits.cast::<u8>(), (w * h * 4) as usize)
            }
        }
    }

    impl Drop for Grabber {
        fn drop(&mut self) {
            // SAFETY: releasing the objects created in `new`.
            unsafe {
                SelectObject(self.memory, self.old);
                let _ = DeleteObject(self.dib.into());
                let _ = DeleteDC(self.memory);
                ReleaseDC(None, self.screen);
            }
        }
    }

    fn pack(hi: u32, lo: u32) -> u64 {
        (u64::from(hi) << 32) | u64::from(lo)
    }

    pub fn record_mp4(out: &Path, area: (i32, i32, i32, i32), stop: &AtomicBool) -> AppResult<()> {
        let (w, h) = even(area.2, area.3);
        let grabber = Grabber::new((area.0, area.1, w, h))?;
        encode_mp4(out, (w, h), stop, MP4_MAX_LENGTH, |buf| {
            buf.copy_from_slice(grabber.grab())
        })
    }

    /// Encode frames from `next` (top-down BGRA, `size`) to an MP4 until `stop` or `max`.
    pub fn encode_mp4(
        out: &Path,
        (w, h): (i32, i32),
        stop: &AtomicBool,
        max: Duration,
        mut next: impl FnMut(&mut [u8]),
    ) -> AppResult<()> {
        let fail = |e: windows::core::Error| AppError::Platform(format!("video encoder: {e}"));
        // SAFETY: Media Foundation on this thread, started and shut down here.
        unsafe {
            MFStartup(MF_VERSION, MFSTARTUP_FULL).map_err(fail)?;
            let result = (|| -> windows::core::Result<()> {
                let writer: IMFSinkWriter =
                    MFCreateSinkWriterFromURL(&HSTRING::from(out.as_os_str()), None, None)?;
                let size = pack(w as u32, h as u32);
                let rate = pack(MP4_FPS, 1);
                let output = MFCreateMediaType()?;
                output.SetGUID(&MF_MT_MAJOR_TYPE, &MFMediaType_Video)?;
                output.SetGUID(&MF_MT_SUBTYPE, &MFVideoFormat_H264)?;
                output.SetUINT32(&MF_MT_AVG_BITRATE, bitrate(w, h, MP4_FPS))?;
                output.SetUINT32(&MF_MT_INTERLACE_MODE, MFVideoInterlace_Progressive.0 as u32)?;
                output.SetUINT64(&MF_MT_FRAME_SIZE, size)?;
                output.SetUINT64(&MF_MT_FRAME_RATE, rate)?;
                output.SetUINT64(&MF_MT_PIXEL_ASPECT_RATIO, pack(1, 1))?;
                let index = writer.AddStream(&output)?;
                let input = MFCreateMediaType()?;
                input.SetGUID(&MF_MT_MAJOR_TYPE, &MFMediaType_Video)?;
                input.SetGUID(&MF_MT_SUBTYPE, &MFVideoFormat_RGB32)?;
                input.SetUINT32(&MF_MT_INTERLACE_MODE, MFVideoInterlace_Progressive.0 as u32)?;
                input.SetUINT64(&MF_MT_FRAME_SIZE, size)?;
                input.SetUINT64(&MF_MT_FRAME_RATE, rate)?;
                input.SetUINT64(&MF_MT_PIXEL_ASPECT_RATIO, pack(1, 1))?;
                // Positive stride: rows top-down, as GDI gives them.
                input.SetUINT32(&MF_MT_DEFAULT_STRIDE, (w * 4) as u32)?;
                writer.SetInputMediaType(index, &input, None)?;
                writer.BeginWriting()?;
                let started = Instant::now();
                let frame = Duration::from_secs(1) / MP4_FPS;
                let len = (w * h * 4) as u32;
                let mut last_time = 0i64;
                while !stop.load(Ordering::SeqCst) && started.elapsed() < max {
                    let at = Instant::now();
                    let buffer = MFCreateMemoryBuffer(len)?;
                    let mut ptr = std::ptr::null_mut();
                    buffer.Lock(&mut ptr, None, None)?;
                    next(std::slice::from_raw_parts_mut(ptr, len as usize));
                    buffer.Unlock()?;
                    buffer.SetCurrentLength(len)?;
                    let sample = MFCreateSample()?;
                    sample.AddBuffer(&buffer)?;
                    // Real time, in 100 ns units, so a slow frame doesn't speed the video up.
                    let time = (started.elapsed().as_nanos() / 100) as i64;
                    sample.SetSampleTime(time)?;
                    sample.SetSampleDuration((time - last_time).max(1))?;
                    last_time = time;
                    writer.WriteSample(index, &sample)?;
                    if let Some(rest) = frame.checked_sub(at.elapsed()) {
                        std::thread::sleep(rest);
                    }
                }
                writer.Finalize()
            })();
            let _ = MFShutdown();
            result.map_err(fail)
        }
    }

    pub fn record_gif(out: &Path, area: (i32, i32, i32, i32), stop: &AtomicBool) -> AppResult<()> {
        use image::{codecs::gif::GifEncoder, Delay, Frame, RgbaImage};

        let (w, h) = (area.2.max(1) as u32, area.3.max(1) as u32);
        let grabber = Grabber::new(area)?;
        let (gw, gh) = gif_size(w, h);
        let (tx, rx) = std::sync::mpsc::sync_channel::<RgbaImage>(4);
        let file = std::fs::File::create(out)?;
        // Encoding (colour reduction) is slow: it runs beside the capture, which drops frames
        // rather than fall behind.
        let encoder = std::thread::spawn(move || -> AppResult<()> {
            let mut gif = GifEncoder::new_with_speed(std::io::BufWriter::new(file), 20);
            gif.set_repeat(image::codecs::gif::Repeat::Infinite)
                .map_err(|e| AppError::Platform(e.to_string()))?;
            for img in rx {
                gif.encode_frame(Frame::from_parts(
                    img,
                    0,
                    0,
                    Delay::from_numer_denom_ms(1000 / GIF_FPS, 1),
                ))
                .map_err(|e| AppError::Platform(e.to_string()))?;
            }
            Ok(())
        });
        let started = Instant::now();
        let frame = Duration::from_secs(1) / GIF_FPS;
        while !stop.load(Ordering::SeqCst) && started.elapsed() < GIF_MAX_LENGTH {
            let at = Instant::now();
            let mut rgba = grabber.grab().to_vec();
            for px in rgba.chunks_exact_mut(4) {
                px.swap(0, 2);
                px[3] = 255;
            }
            if let Some(full) = RgbaImage::from_raw(w, h, rgba) {
                let small = image::imageops::resize(&full, gw, gh, image::imageops::FilterType::Triangle);
                let _ = tx.try_send(small);
            }
            if let Some(rest) = frame.checked_sub(at.elapsed()) {
                std::thread::sleep(rest);
            }
        }
        drop(tx);
        encoder
            .join()
            .map_err(|_| AppError::Platform("the GIF encoder crashed".into()))?
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Windows' H.264 encoder takes our RGB frames (synthetic ones: no screen is captured).
    #[cfg(windows)]
    #[test]
    fn encodes_frames_to_mp4() {
        let out = std::env::temp_dir().join(format!("overline-rec-test-{}.mp4", std::process::id()));
        let stop = AtomicBool::new(false);
        let mut n = 0u8;
        win::encode_mp4(&out, (320, 240), &stop, Duration::from_millis(600), |buf| {
            n = n.wrapping_add(9);
            buf.fill(n);
        })
        .unwrap();
        let size = std::fs::metadata(&out).map(|m| m.len()).unwrap_or(0);
        let _ = std::fs::remove_file(&out);
        assert!(size > 1000, "mp4 is {size} bytes");
    }

    #[test]
    fn sizes_and_rates() {
        assert_eq!(even(1919, 1081), (1918, 1080));
        assert_eq!(gif_size(1920, 1080), (800, 450));
        assert_eq!(gif_size(640, 480), (640, 480));
        assert_eq!(bitrate(1920, 1080, 20), 6_220_800);
        assert_eq!(bitrate(10, 10, 1), 1_500_000);
    }
}
