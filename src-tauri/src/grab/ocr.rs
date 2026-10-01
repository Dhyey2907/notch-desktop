//! Windows' built-in text recognition (Windows.Media.Ocr), in the user's languages.

use windows::{
    Graphics::Imaging::{BitmapAlphaMode, BitmapPixelFormat, SoftwareBitmap},
    Media::Ocr::OcrEngine,
    Storage::Streams::DataWriter,
};

use crate::error::{AppError, AppResult};

/// Text in a BGRA image, one line per line found.
pub fn read(width: u32, height: u32, bgra: &[u8]) -> AppResult<String> {
    let fail = |e: windows::core::Error| AppError::Platform(e.to_string());
    let engine = OcrEngine::TryCreateFromUserProfileLanguages().map_err(|_| {
        AppError::InvalidInput("Windows has no text recognition for your languages. Add a language pack in Settings → Time & language.".into())
    })?;
    let max = OcrEngine::MaxImageDimension().map_err(fail)?;
    // Too big for the engine: read the middle part rather than fail.
    let (w, h) = (width.min(max), height.min(max));
    let mut pixels = Vec::with_capacity((w * h * 4) as usize);
    for row in 0..h {
        let start = (row * width * 4) as usize;
        pixels.extend_from_slice(&bgra[start..start + (w * 4) as usize]);
    }
    let writer = DataWriter::new().map_err(fail)?;
    writer.WriteBytes(&pixels).map_err(fail)?;
    let buffer = writer.DetachBuffer().map_err(fail)?;
    let bitmap = SoftwareBitmap::CreateCopyWithAlphaFromBuffer(
        &buffer,
        BitmapPixelFormat::Bgra8,
        w as i32,
        h as i32,
        BitmapAlphaMode::Premultiplied,
    )
    .map_err(fail)?;
    let result = engine
        .RecognizeAsync(&bitmap)
        .and_then(|op| op.get())
        .map_err(fail)?;
    let lines: Vec<String> = result
        .Lines()
        .map_err(fail)?
        .into_iter()
        .filter_map(|line| line.Text().ok().map(|t| t.to_string()))
        .collect();
    Ok(lines.join("\n"))
}
