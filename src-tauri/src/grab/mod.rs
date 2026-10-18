//! Screen text grabber: freeze the screen under the pointer, let the user draw a box, read the
//! text inside with Windows' built-in OCR (on this PC; nothing is uploaded) and copy it.

#[cfg(windows)]
pub(crate) mod capture;
#[cfg(windows)]
pub(crate) mod ocr;
pub mod service;

/// A captured screen: BGRA pixels, top-down.
#[derive(Clone)]
pub struct Shot {
    pub width: u32,
    pub height: u32,
    pub bgra: Vec<u8>,
    /// Where the screen is on the desktop (physical pixels).
    pub x: i32,
    pub y: i32,
}

/// Cut `(x, y, w, h)` out of a shot, clamped to its edges.
pub fn crop(shot: &Shot, x: u32, y: u32, w: u32, h: u32) -> Option<(u32, u32, Vec<u8>)> {
    let x = x.min(shot.width);
    let y = y.min(shot.height);
    let w = w.min(shot.width - x);
    let h = h.min(shot.height - y);
    if w < 4 || h < 4 {
        return None;
    }
    let stride = shot.width as usize * 4;
    let mut out = Vec::with_capacity(w as usize * h as usize * 4);
    for row in y..y + h {
        let start = row as usize * stride + x as usize * 4;
        out.extend_from_slice(&shot.bgra[start..start + w as usize * 4]);
    }
    Some((w, h, out))
}

/// OCR reads small text better a bit larger: scale up tiny crops (nearest-neighbour ×2).
pub fn upscale2(w: u32, h: u32, bgra: &[u8]) -> (u32, u32, Vec<u8>) {
    let (nw, nh) = (w * 2, h * 2);
    let mut out = vec![0u8; nw as usize * nh as usize * 4];
    for y in 0..nh {
        for x in 0..nw {
            let src = ((y / 2) * w + x / 2) as usize * 4;
            let dst = (y * nw + x) as usize * 4;
            out[dst..dst + 4].copy_from_slice(&bgra[src..src + 4]);
        }
    }
    (nw, nh, out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn shot() -> Shot {
        let (w, h) = (10u32, 8u32);
        let bgra = (0..w * h).flat_map(|i| [i as u8, 0, 0, 255]).collect();
        Shot {
            width: w,
            height: h,
            bgra,
            x: 0,
            y: 0,
        }
    }

    #[test]
    fn crops_inside_the_shot() {
        let (w, h, px) = crop(&shot(), 2, 3, 4, 4).unwrap();
        assert_eq!((w, h), (4, 4));
        assert_eq!(px[0], 32); // row 3, column 2
        let (w, h, _) = crop(&shot(), 8, 2, 50, 50).unwrap_or((0, 0, vec![]));
        assert_eq!((w, h), (0, 0)); // only 2 px wide: too small
        assert!(crop(&shot(), 0, 0, 2, 2).is_none());
        let (w, h, _) = crop(&shot(), 5, 2, 50, 50).unwrap();
        assert_eq!((w, h), (5, 6));
    }

    #[test]
    fn upscales_pixels() {
        let (w, h, px) = upscale2(1, 1, &[1, 2, 3, 4]);
        assert_eq!((w, h), (2, 2));
        assert_eq!(px, [1, 2, 3, 4].repeat(4));
    }
}
