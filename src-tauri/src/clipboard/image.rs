//! Clipboard bitmaps: Windows `CF_DIB` (a BITMAPINFOHEADER followed by pixels) to PNG for
//! history thumbnails, and PNG back to `CF_DIB` when a picture is copied again.

use std::io::Cursor;

/// Pictures bigger than this (in pixels) aren't kept; ~40 MP covers 8K screenshots.
pub const MAX_PIXELS: u64 = 40_000_000;

const BI_RGB: u32 = 0;
const BI_BITFIELDS: u32 = 3;

fn u16_at(b: &[u8], at: usize) -> Option<u16> {
    Some(u16::from_le_bytes(b.get(at..at + 2)?.try_into().ok()?))
}
fn u32_at(b: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(b.get(at..at + 4)?.try_into().ok()?))
}
fn i32_at(b: &[u8], at: usize) -> Option<i32> {
    Some(i32::from_le_bytes(b.get(at..at + 4)?.try_into().ok()?))
}

/// A `CF_DIB` (24- or 32-bit, uncompressed) as PNG bytes plus its size.
pub fn dib_to_png(dib: &[u8]) -> Option<(Vec<u8>, u32, u32)> {
    let header = u32_at(dib, 0)? as usize;
    let width = i32_at(dib, 4)?;
    let height = i32_at(dib, 8)?;
    let bits = u16_at(dib, 14)?;
    let compression = u32_at(dib, 16)?;
    let colors_used = u32_at(dib, 32)? as usize;
    if header < 40 || width <= 0 || height == 0 || !matches!(bits, 24 | 32) {
        return None;
    }
    if !(compression == BI_RGB || (compression == BI_BITFIELDS && bits == 32)) {
        return None;
    }
    let (w, h) = (width as u32, height.unsigned_abs());
    if u64::from(w) * u64::from(h) > MAX_PIXELS {
        return None;
    }
    // A plain 40-byte header keeps its three BITFIELDS masks after it.
    let masks = if compression == BI_BITFIELDS && header == 40 {
        12
    } else {
        0
    };
    let offset = header + masks + colors_used * 4;
    let bytes_per_pixel = usize::from(bits / 8);
    let stride = (w as usize * usize::from(bits)).div_ceil(32) * 4;
    let pixels = dib.get(offset..offset + stride * h as usize)?;

    let mut rgba = Vec::with_capacity(w as usize * h as usize * 4);
    let mut any_alpha = false;
    for y in 0..h as usize {
        // Positive height means the rows are stored bottom-up.
        let row = if height > 0 { h as usize - 1 - y } else { y };
        let line = &pixels[row * stride..row * stride + w as usize * bytes_per_pixel];
        for px in line.chunks_exact(bytes_per_pixel) {
            let alpha = if bits == 32 { px[3] } else { 255 };
            any_alpha |= alpha != 0;
            rgba.extend_from_slice(&[px[2], px[1], px[0], alpha]);
        }
    }
    // Many apps leave the 32-bit alpha byte at zero: that means opaque, not invisible.
    if !any_alpha {
        rgba.chunks_exact_mut(4).for_each(|px| px[3] = 255);
    }
    Some((encode_png(&rgba, w, h)?, w, h))
}

fn encode_png(rgba: &[u8], w: u32, h: u32) -> Option<Vec<u8>> {
    let mut out = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut out, w, h);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().ok()?;
        writer.write_image_data(rgba).ok()?;
    }
    Some(out)
}

/// Width and height of a PNG, without decoding the pixels.
pub fn png_size(png_bytes: &[u8]) -> Option<(u32, u32)> {
    let reader = png::Decoder::new(Cursor::new(png_bytes)).read_info().ok()?;
    let info = reader.info();
    Some((info.width, info.height))
}

/// PNG bytes as a 32-bit bottom-up `CF_DIB`, which every Windows app can paste.
pub fn png_to_dib(png_bytes: &[u8]) -> Option<Vec<u8>> {
    let mut decoder = png::Decoder::new(Cursor::new(png_bytes));
    decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    let mut reader = decoder.read_info().ok()?;
    let (w, h) = (reader.info().width, reader.info().height);
    if u64::from(w) * u64::from(h) > MAX_PIXELS {
        return None;
    }
    let mut buf = vec![0; reader.output_buffer_size()?];
    let frame = reader.next_frame(&mut buf).ok()?;
    let channels = match frame.color_type {
        png::ColorType::Grayscale => 1,
        png::ColorType::GrayscaleAlpha => 2,
        png::ColorType::Rgb => 3,
        png::ColorType::Rgba => 4,
        png::ColorType::Indexed => return None,
    };
    let line = frame.line_size;

    let mut dib = Vec::with_capacity(40 + w as usize * h as usize * 4);
    dib.extend_from_slice(&40u32.to_le_bytes());
    dib.extend_from_slice(&(w as i32).to_le_bytes());
    dib.extend_from_slice(&(h as i32).to_le_bytes()); // bottom-up
    dib.extend_from_slice(&1u16.to_le_bytes()); // planes
    dib.extend_from_slice(&32u16.to_le_bytes());
    dib.extend_from_slice(&BI_RGB.to_le_bytes());
    dib.extend_from_slice(&(w * h * 4).to_le_bytes());
    dib.extend_from_slice(&[0; 16]); // resolution and palette: unused
    for y in (0..h as usize).rev() {
        for px in buf[y * line..y * line + w as usize * channels].chunks_exact(channels) {
            let (r, g, b, a) = match channels {
                1 => (px[0], px[0], px[0], 255),
                2 => (px[0], px[0], px[0], px[1]),
                3 => (px[0], px[1], px[2], 255),
                _ => (px[0], px[1], px[2], px[3]),
            };
            dib.extend_from_slice(&[b, g, r, a]);
        }
    }
    Some(dib)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A 2×2 24-bit bottom-up DIB: top row red, blue; bottom row green, white.
    fn dib24() -> Vec<u8> {
        let mut d = Vec::new();
        d.extend_from_slice(&40u32.to_le_bytes());
        d.extend_from_slice(&2i32.to_le_bytes());
        d.extend_from_slice(&2i32.to_le_bytes());
        d.extend_from_slice(&1u16.to_le_bytes());
        d.extend_from_slice(&24u16.to_le_bytes());
        d.extend_from_slice(&[0; 24]);
        // Rows are padded to 4 bytes (6 pixel bytes + 2). Bottom row first.
        d.extend_from_slice(&[0, 255, 0, 255, 255, 255, 0, 0]);
        d.extend_from_slice(&[0, 0, 255, 255, 0, 0, 0, 0]);
        d
    }

    fn rgba_of(png_bytes: &[u8]) -> Vec<u8> {
        let mut reader = png::Decoder::new(Cursor::new(png_bytes)).read_info().unwrap();
        let mut buf = vec![0; reader.output_buffer_size().unwrap()];
        reader.next_frame(&mut buf).unwrap();
        buf
    }

    #[test]
    fn converts_a_bottom_up_24_bit_dib() {
        let (png_bytes, w, h) = dib_to_png(&dib24()).unwrap();
        assert_eq!((w, h), (2, 2));
        assert_eq!(
            rgba_of(&png_bytes),
            [255, 0, 0, 255, 0, 0, 255, 255, 0, 255, 0, 255, 255, 255, 255, 255]
        );
        assert_eq!(png_size(&png_bytes), Some((2, 2)));
    }

    #[test]
    fn round_trips_through_png() {
        let (png_bytes, _, _) = dib_to_png(&dib24()).unwrap();
        let dib = png_to_dib(&png_bytes).unwrap();
        let (again, _, _) = dib_to_png(&dib).unwrap();
        assert_eq!(rgba_of(&again), rgba_of(&png_bytes));
    }

    #[test]
    fn treats_zero_alpha_32_bit_as_opaque() {
        let mut d = dib24();
        d[14] = 32;
        d.truncate(40);
        d.extend_from_slice(&[0, 0, 255, 0, 255, 0, 0, 0]); // bottom row: red, blue
        d.extend_from_slice(&[0, 255, 0, 0, 255, 255, 255, 0]); // top row: green, white
        let (png_bytes, _, _) = dib_to_png(&d).unwrap();
        assert!(rgba_of(&png_bytes).chunks(4).all(|px| px[3] == 255));
    }

    #[test]
    fn rejects_unsupported_or_truncated_bitmaps() {
        let mut d = dib24();
        d[14] = 8; // palette images
        assert!(dib_to_png(&d).is_none());
        assert!(dib_to_png(&dib24()[..50]).is_none());
        assert!(dib_to_png(&[]).is_none());
    }
}
