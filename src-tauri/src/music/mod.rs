//! Music-reactive notch: a beat/loudness signal from what's playing (output level meters; only a
//! level number leaves the audio thread — nothing is recorded), colors from the album art, and
//! time-synced lyrics from LRCLIB.

#[cfg(windows)]
mod loopback;
pub mod service;

use std::{collections::VecDeque, io::Cursor};

use serde::Serialize;
use ts_rs::TS;

/// One line of time-synced lyrics.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct LyricLine {
    #[ts(type = "number")]
    pub at_ms: u64,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Lyrics {
    /// "title — artist" the lines belong to, so a late answer isn't shown for the next song.
    pub track: String,
    /// Empty when the song has no synced lyrics (or it's instrumental).
    pub lines: Vec<LyricLine>,
}

/// Two colors from the album art, as `#rrggbb`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Palette {
    pub primary: String,
    pub accent: String,
}

pub fn track_key(title: &str, artist: &str) -> String {
    format!("{} — {}", title.trim(), artist.trim())
}

/// Parse LRC (`[mm:ss.xx] text`, possibly several stamps per line) into sorted lines.
pub fn parse_lrc(lrc: &str) -> Vec<LyricLine> {
    let mut lines = Vec::new();
    for raw in lrc.lines() {
        let mut rest = raw.trim();
        let mut stamps = Vec::new();
        while let Some(after) = rest.strip_prefix('[') {
            let Some((stamp, tail)) = after.split_once(']') else {
                break;
            };
            match parse_stamp(stamp) {
                Some(ms) => stamps.push(ms),
                None => break, // a tag like [ar: Artist]
            }
            rest = tail;
        }
        let text: String = rest.trim().chars().take(200).collect();
        for at_ms in stamps {
            lines.push(LyricLine {
                at_ms,
                text: text.clone(),
            });
        }
    }
    lines.sort_by_key(|l| l.at_ms);
    lines
}

fn parse_stamp(stamp: &str) -> Option<u64> {
    let (min, sec) = stamp.split_once(':')?;
    let min: u64 = min.trim().parse().ok()?;
    let sec: f64 = sec.trim().parse().ok()?;
    (sec.is_finite() && (0.0..60.0).contains(&sec)).then(|| min * 60_000 + (sec * 1000.0).round() as u64)
}

/// From LRCLIB search results, the synced lyrics whose length is closest to the song playing
/// (within a few seconds when its length is known).
pub fn pick_search(results: &serde_json::Value, duration_ms: Option<u64>) -> Option<String> {
    let target = duration_ms.map(|ms| ms as f64 / 1000.0);
    results
        .as_array()?
        .iter()
        .filter_map(|r| {
            let synced = r.get("syncedLyrics")?.as_str()?.trim();
            if synced.is_empty() {
                return None;
            }
            let off = match (target, r.get("duration").and_then(serde_json::Value::as_f64)) {
                (Some(t), Some(d)) => (t - d).abs(),
                (Some(_), None) => f64::MAX / 2.0,
                (None, _) => 0.0,
            };
            (target.is_none() || off <= 4.0).then_some((off, synced))
        })
        .min_by(|a, b| a.0.total_cmp(&b.0))
        .map(|(_, s)| s.to_owned())
}

/// Beat detector: loudness against the last second's average; a beat is a jump well above it.
pub struct BeatDetector {
    history: VecDeque<f32>,
    peak: f32,
    since_beat_ms: u32,
}

/// Chunks of energy kept (~1 s at ~23 ms per chunk).
const HISTORY: usize = 43;
const BEAT_RATIO: f32 = 1.35;
const MIN_ENERGY: f32 = 0.0004;
const MIN_BEAT_GAP_MS: u32 = 230;

impl Default for BeatDetector {
    fn default() -> Self {
        Self {
            history: VecDeque::with_capacity(HISTORY),
            peak: 0.01,
            since_beat_ms: u32::MAX / 2,
        }
    }
}

impl BeatDetector {
    /// Feed one chunk of samples (mono, -1..1) covering `chunk_ms`. Returns (level 0..1, beat).
    pub fn feed(&mut self, samples: &[f32], chunk_ms: u32) -> (f32, bool) {
        let energy = if samples.is_empty() {
            0.0
        } else {
            samples.iter().map(|s| s * s).sum::<f32>() / samples.len() as f32
        };
        let average = if self.history.is_empty() {
            energy
        } else {
            self.history.iter().sum::<f32>() / self.history.len() as f32
        };
        self.since_beat_ms = self.since_beat_ms.saturating_add(chunk_ms);
        let beat = self.history.len() >= HISTORY / 2
            && energy > MIN_ENERGY
            && energy > average * BEAT_RATIO
            && self.since_beat_ms >= MIN_BEAT_GAP_MS;
        if beat {
            self.since_beat_ms = 0;
        }
        if self.history.len() == HISTORY {
            self.history.pop_front();
        }
        self.history.push_back(energy);
        // Level relative to a slowly decaying recent peak, so quiet and loud songs both move.
        let rms = energy.sqrt();
        self.peak = (self.peak * 0.995).max(rms).max(0.01);
        ((rms / self.peak).clamp(0.0, 1.0), beat)
    }
}

/// Dominant color and a vivid accent from PNG album art.
pub fn palette(png_bytes: &[u8]) -> Option<Palette> {
    let mut decoder = png::Decoder::new(Cursor::new(png_bytes));
    decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    let mut reader = decoder.read_info().ok()?;
    let (w, h) = (reader.info().width as usize, reader.info().height as usize);
    if w == 0 || h == 0 || w * h > 16_000_000 {
        return None;
    }
    let mut buf = vec![0; reader.output_buffer_size()?];
    let frame = reader.next_frame(&mut buf).ok()?;
    let channels = match frame.color_type {
        png::ColorType::Rgb => 3,
        png::ColorType::Rgba => 4,
        _ => return None,
    };
    // ~2000 samples, whatever the size.
    let step = ((w * h) / 2000).max(1);
    let pixels: Vec<[f32; 3]> = buf[..w * h * channels]
        .chunks_exact(channels)
        .step_by(step)
        .filter(|p| channels == 3 || p[3] > 128)
        .map(|p| [f32::from(p[0]), f32::from(p[1]), f32::from(p[2])])
        .collect();
    if pixels.is_empty() {
        return None;
    }
    let clusters = kmeans(&pixels, 4);
    let primary = clusters.iter().max_by_key(|(_, n)| *n)?.0;
    // Accent: the most saturated cluster that isn't nearly black or white.
    let accent = clusters
        .iter()
        .filter(|(c, n)| *n > 0 && lightness(c) > 0.18 && lightness(c) < 0.9)
        .max_by(|a, b| saturation(&a.0).total_cmp(&saturation(&b.0)))
        .map(|(c, _)| *c)
        .unwrap_or(primary);
    Some(Palette {
        primary: hex(primary),
        accent: hex(accent),
    })
}

fn kmeans(pixels: &[[f32; 3]], k: usize) -> Vec<([f32; 3], usize)> {
    let mut centers: Vec<[f32; 3]> = (0..k).map(|i| pixels[i * pixels.len() / k]).collect();
    let mut counts = vec![0; k];
    for _ in 0..8 {
        let mut sums = vec![[0.0f32; 3]; k];
        counts = vec![0; k];
        for p in pixels {
            let i = (0..k)
                .min_by(|&a, &b| dist(p, &centers[a]).total_cmp(&dist(p, &centers[b])))
                .unwrap_or(0);
            for c in 0..3 {
                sums[i][c] += p[c];
            }
            counts[i] += 1;
        }
        for i in 0..k {
            if counts[i] > 0 {
                centers[i] = sums[i].map(|s| s / counts[i] as f32);
            }
        }
    }
    centers.into_iter().zip(counts).collect()
}

fn dist(a: &[f32; 3], b: &[f32; 3]) -> f32 {
    (0..3).map(|i| (a[i] - b[i]).powi(2)).sum()
}

fn lightness(c: &[f32; 3]) -> f32 {
    let (max, min) = (c[0].max(c[1]).max(c[2]), c[0].min(c[1]).min(c[2]));
    (max + min) / 510.0
}

fn saturation(c: &[f32; 3]) -> f32 {
    let (max, min) = (c[0].max(c[1]).max(c[2]), c[0].min(c[1]).min(c[2]));
    if max == 0.0 {
        0.0
    } else {
        (max - min) / max
    }
}

fn hex(c: [f32; 3]) -> String {
    format!(
        "#{:02x}{:02x}{:02x}",
        c[0].round() as u8,
        c[1].round() as u8,
        c[2].round() as u8
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_synced_lyrics() {
        let lrc = "[ar: Someone]\n[00:12.50] First line\n[00:05.00][01:05.00] Chorus\n[00:20.5]\nnot a line";
        let lines = parse_lrc(lrc);
        let at: Vec<u64> = lines.iter().map(|l| l.at_ms).collect();
        assert_eq!(at, [5_000, 12_500, 20_500, 65_000]);
        assert_eq!(lines[0].text, "Chorus");
        assert_eq!(lines[2].text, "", "an empty line is a pause");
        assert!(parse_lrc("[99:99.00] bad").is_empty());
    }

    #[test]
    fn picks_the_closest_synced_version() {
        let results = serde_json::json!([
            { "duration": 180.0, "syncedLyrics": "[00:01.00] short edit" },
            { "duration": 203.0, "syncedLyrics": "[00:01.00] album" },
            { "duration": 202.0, "syncedLyrics": "" },
            { "duration": 240.0, "syncedLyrics": "[00:01.00] live" },
        ]);
        assert_eq!(
            pick_search(&results, Some(202_500)).as_deref(),
            Some("[00:01.00] album")
        );
        assert_eq!(pick_search(&results, Some(100_000)), None, "nothing close enough");
        assert!(pick_search(&results, None).is_some());
    }

    #[test]
    fn finds_beats_in_a_pulsing_signal() {
        let mut detector = BeatDetector::default();
        let quiet = vec![0.05f32; 1000];
        let loud = vec![0.6f32; 1000];
        let mut beats = 0;
        let mut levels = Vec::new();
        for i in 0..200 {
            // A kick roughly every 500 ms (every 22nd chunk of ~23 ms).
            let chunk = if i % 22 == 0 { &loud } else { &quiet };
            let (level, beat) = detector.feed(chunk, 23);
            beats += usize::from(beat);
            levels.push(level);
        }
        assert!((7..=9).contains(&beats), "found {beats} beats");
        assert!(levels.iter().all(|l| (0.0..=1.0).contains(l)));
    }

    #[test]
    fn no_beats_in_silence_or_steady_noise() {
        let mut detector = BeatDetector::default();
        let steady = vec![0.3f32; 1000];
        assert!((0..200).all(|_| !detector.feed(&steady, 23).1));
        let mut detector = BeatDetector::default();
        assert!((0..200).all(|_| !detector.feed(&[0.0; 1000], 23).1));
    }

    #[test]
    fn extracts_colors_from_art() {
        // 20×20: mostly dark blue with a bright orange stripe.
        let (w, h) = (20u32, 20u32);
        let mut rgba = Vec::new();
        for y in 0..h {
            for _ in 0..w {
                rgba.extend_from_slice(if y < 5 {
                    &[240, 120, 20, 255]
                } else {
                    &[20, 30, 90, 255]
                });
            }
        }
        let mut png_bytes = Vec::new();
        {
            let mut e = png::Encoder::new(&mut png_bytes, w, h);
            e.set_color(png::ColorType::Rgba);
            e.set_depth(png::BitDepth::Eight);
            e.write_header().unwrap().write_image_data(&rgba).unwrap();
        }
        let p = palette(&png_bytes).unwrap();
        assert_eq!(p.primary, "#141e5a");
        assert_eq!(p.accent, "#f07814");
        assert!(palette(b"not a png").is_none());
    }
}
