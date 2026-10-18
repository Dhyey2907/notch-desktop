//! Meeting notes: record a meeting (your microphone and what the PC plays, so both sides of a
//! call), transcribe it on this PC with Whisper (whisper.cpp; nothing is uploaded), then have
//! the AI write a summary and action items. The notes go to Notes, the action items to To-do,
//! and the full transcript is saved next to the recording.

pub mod service;
#[cfg(windows)]
pub mod whisper;

use std::{
    fs::File,
    io::{BufWriter, Read, Write},
    path::Path,
};

use serde::Serialize;
use ts_rs::TS;

use crate::error::{AppError, AppResult};

/// Whisper wants 16 kHz mono.
pub const WHISPER_RATE: u32 = 16_000;
/// The longest transcript the AI is given (about two hours of talk).
pub const MAX_TRANSCRIPT_CHARS: usize = 60_000;

pub const SUMMARY_SYSTEM: &str = "You write meeting notes from a transcript (it may have recognition \
mistakes; use common sense). Write: a one-line title, then 'Summary:' with 3-6 short bullet points, \
then 'Action items:' with one line per task as '- [ ] who: what (when)' (leave out who/when if not \
said). If there are no action items write 'Action items: none'. Plain text, no other headers.";

/// What the meeting recorder is doing.
#[derive(Debug, Clone, Default, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct MeetingState {
    /// "idle", "recording", "transcribing", "summarizing", "installing".
    pub phase: String,
    /// When recording started (Unix ms).
    #[ts(optional, type = "number")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub started_at: Option<i64>,
    /// Whisper and its speech model are downloaded.
    pub ready: bool,
    /// Download progress 0–100 while installing.
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub progress: Option<u8>,
    /// The last problem, in plain words.
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// A 16-bit PCM WAV's rate, channels and samples.
pub fn read_wav(path: &Path) -> AppResult<(u32, u16, Vec<i16>)> {
    let mut bytes = Vec::new();
    File::open(path)?.read_to_end(&mut bytes)?;
    parse_wav(&bytes).ok_or_else(|| AppError::Platform("couldn't read the recording".into()))
}

fn parse_wav(bytes: &[u8]) -> Option<(u32, u16, Vec<i16>)> {
    if bytes.len() < 44 || &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
        return None;
    }
    let mut at = 12;
    let (mut rate, mut channels) = (0u32, 0u16);
    while at + 8 <= bytes.len() {
        let id = &bytes[at..at + 4];
        let size = u32::from_le_bytes(bytes[at + 4..at + 8].try_into().ok()?) as usize;
        let body = at + 8;
        if id == b"fmt " && body + 16 <= bytes.len() {
            channels = u16::from_le_bytes(bytes[body + 2..body + 4].try_into().ok()?);
            rate = u32::from_le_bytes(bytes[body + 4..body + 8].try_into().ok()?);
        } else if id == b"data" {
            let end = (body + size).min(bytes.len());
            let samples = bytes[body..end]
                .chunks_exact(2)
                .map(|c| i16::from_le_bytes([c[0], c[1]]))
                .collect();
            return (rate > 0 && channels > 0).then_some((rate, channels, samples));
        }
        at = body + size + (size & 1);
    }
    None
}

/// Interleaved samples → 16 kHz mono floats (channels averaged, linear resampling).
pub fn to_mono_16k(samples: &[i16], rate: u32, channels: u16) -> Vec<f32> {
    let ch = usize::from(channels.max(1));
    let mono: Vec<f32> = samples
        .chunks_exact(ch)
        .map(|f| f.iter().map(|&s| f32::from(s) / 32768.0).sum::<f32>() / ch as f32)
        .collect();
    if rate == WHISPER_RATE || mono.is_empty() {
        return mono;
    }
    let ratio = f64::from(rate) / f64::from(WHISPER_RATE);
    let out_len = (mono.len() as f64 / ratio).floor() as usize;
    (0..out_len)
        .map(|i| {
            let pos = i as f64 * ratio;
            let j = pos.floor() as usize;
            let t = (pos - j as f64) as f32;
            let a = mono[j];
            let b = *mono.get(j + 1).unwrap_or(&a);
            a + (b - a) * t
        })
        .collect()
}

/// Two tracks mixed into one (the shorter padded with silence), kept from clipping.
pub fn mix(a: &[f32], b: &[f32]) -> Vec<f32> {
    (0..a.len().max(b.len()))
        .map(|i| (a.get(i).copied().unwrap_or(0.0) + b.get(i).copied().unwrap_or(0.0)).clamp(-1.0, 1.0))
        .collect()
}

/// Write 16 kHz mono floats as a 16-bit WAV (what Whisper reads).
pub fn write_wav_16k(path: &Path, samples: &[f32]) -> AppResult<()> {
    let mut out = BufWriter::new(File::create(path)?);
    let data = (samples.len() * 2) as u32;
    out.write_all(b"RIFF")?;
    out.write_all(&(36 + data).to_le_bytes())?;
    out.write_all(b"WAVEfmt ")?;
    out.write_all(&16u32.to_le_bytes())?;
    out.write_all(&1u16.to_le_bytes())?;
    out.write_all(&1u16.to_le_bytes())?;
    out.write_all(&WHISPER_RATE.to_le_bytes())?;
    out.write_all(&(WHISPER_RATE * 2).to_le_bytes())?;
    out.write_all(&2u16.to_le_bytes())?;
    out.write_all(&16u16.to_le_bytes())?;
    out.write_all(b"data")?;
    out.write_all(&data.to_le_bytes())?;
    for s in samples {
        out.write_all(&((s.clamp(-1.0, 1.0) * 32767.0) as i16).to_le_bytes())?;
    }
    out.flush()?;
    Ok(())
}

/// The "- [ ] …" lines of the notes, as task titles.
pub fn action_items(notes: &str) -> Vec<String> {
    notes
        .lines()
        .filter_map(|l| {
            let l = l.trim();
            l.strip_prefix("- [ ]")
                .or_else(|| l.strip_prefix("-[ ]"))
                .or_else(|| l.strip_prefix("* [ ]"))
        })
        .map(|t| t.trim().to_string())
        .filter(|t| !t.is_empty() && t.chars().count() <= 200)
        .take(15)
        .collect()
}

/// Whisper marks stretches without speech like "[BLANK_AUDIO]": drop them and empty lines.
pub fn clean_transcript(raw: &str) -> String {
    raw.lines()
        .map(str::trim)
        .filter(|l| {
            !l.is_empty()
                && !(l.starts_with('[') && l.ends_with(']'))
                && !(l.starts_with('(') && l.ends_with(')'))
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resamples_and_mixes_down_to_16k_mono() {
        // 48 kHz stereo, 0.1 s: both channels at half volume.
        let stereo: Vec<i16> = std::iter::repeat_n(16384, 4800 * 2).collect();
        let mono = to_mono_16k(&stereo, 48_000, 2);
        assert_eq!(mono.len(), 1600);
        assert!((mono[10] - 0.5).abs() < 0.001);
        let mixed = mix(&mono, &[0.75; 10]);
        assert_eq!(mixed.len(), 1600);
        assert_eq!(mixed[0], 1.0, "clamped");
        assert!((mixed[100] - 0.5).abs() < 0.001);
    }

    #[test]
    fn wav_round_trip() {
        let path = std::env::temp_dir().join(format!("overline-meeting-{}.wav", std::process::id()));
        write_wav_16k(&path, &[0.0, 0.5, -0.5, 1.0]).unwrap();
        let (rate, ch, samples) = read_wav(&path).unwrap();
        let _ = std::fs::remove_file(&path);
        assert_eq!((rate, ch), (16_000, 1));
        assert_eq!(samples, vec![0, 16383, -16383, 32767]);
    }

    #[test]
    fn finds_action_items() {
        let notes = "Budget review\nSummary:\n- spend is up\nAction items:\n- [ ] Rahul: send the invoice (Friday)\n- [ ] book the room\n- not a task";
        assert_eq!(
            action_items(notes),
            vec!["Rahul: send the invoice (Friday)", "book the room"]
        );
        assert!(action_items("Action items: none").is_empty());
    }

    #[test]
    fn drops_whisper_markers() {
        assert_eq!(
            clean_transcript("[BLANK_AUDIO]\n Hello all.\n\n(music)\nLet's start."),
            "Hello all.\nLet's start."
        );
    }
}
