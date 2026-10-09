//! Fun: PC lights that follow the notch (OpenRGB) and the pet walking along the taskbar
//! while you're away.

pub mod buddy;
pub mod openrgb;
pub mod service;

use serde::Serialize;
use ts_rs::TS;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct RgbStatus {
    /// `off`, `notRunning` (OpenRGB or its SDK server isn't on) or `connected`.
    pub state: String,
    pub devices: u32,
}

/// `#8ab4ff` → (138, 180, 255).
pub fn parse_hex(hex: &str) -> Option<(u8, u8, u8)> {
    let h = hex.strip_prefix('#')?;
    if h.len() != 6 {
        return None;
    }
    let v = u32::from_str_radix(h, 16).ok()?;
    Some(((v >> 16) as u8, (v >> 8) as u8, v as u8))
}

/// The color to show: dim at rest, brighter with the music's loudness, a flash on the beat.
pub fn shade((r, g, b): (u8, u8, u8), level: f32, beat: bool) -> (u8, u8, u8) {
    let k = if beat {
        1.0
    } else {
        (0.35 + level.clamp(0.0, 1.0) * 0.55).min(1.0)
    };
    let f = |c: u8| (f32::from(c) * k).round() as u8;
    (f(r), f(g), f(b))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_colors_and_shades_them() {
        assert_eq!(parse_hex("#8ab4ff"), Some((138, 180, 255)));
        assert_eq!(parse_hex("8ab4ff"), None);
        assert_eq!(parse_hex("#fff"), None);
        assert_eq!(shade((200, 100, 0), 0.0, false), (70, 35, 0));
        assert_eq!(shade((200, 100, 0), 1.0, false), (180, 90, 0));
        assert_eq!(shade((200, 100, 0), 0.2, true), (200, 100, 0));
    }
}
