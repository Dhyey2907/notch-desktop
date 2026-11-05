//! Internet speed test against Cloudflare's public speed test servers (speed.cloudflare.com):
//! latency from a few tiny requests, then a timed download and a timed upload of throwaway
//! bytes. Nothing is saved to disk; only the result is kept.

use std::time::{Duration, Instant};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

const BASE: &str = "https://speed.cloudflare.com";
const PINGS: usize = 5;
const DOWN_BYTES: u64 = 25_000_000;
const UP_BYTES: usize = 8_000_000;
const UP_FIRST_PIECE: usize = 256 * 1024;
const UP_MAX_PIECE: usize = 4 * 1024 * 1024;
/// Each half stops after this long, even on a slow line.
const PHASE_LIMIT: Duration = Duration::from_secs(12);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum SpeedPhase {
    Ping,
    Download,
    Upload,
    Done,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SpeedTest {
    pub phase: SpeedPhase,
    #[ts(optional)]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ping_ms: Option<u32>,
    /// Megabits per second.
    #[ts(optional)]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub down_mbps: Option<f64>,
    #[ts(optional)]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub up_mbps: Option<f64>,
    pub at: DateTime<Utc>,
}

pub fn mbps(bytes: u64, elapsed: Duration) -> f64 {
    let secs = elapsed.as_secs_f64();
    if secs <= 0.0 {
        return 0.0;
    }
    ((bytes as f64 * 8.0 / secs / 1_000_000.0) * 10.0).round() / 10.0
}

pub fn client() -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(PHASE_LIMIT + Duration::from_secs(8))
        .user_agent("Overline")
        .build()
        .unwrap_or_default()
}

/// Lowest of a few round trips, in milliseconds.
pub async fn ping(http: &reqwest::Client) -> Option<u32> {
    let mut best: Option<u32> = None;
    for _ in 0..PINGS {
        let started = Instant::now();
        let ok = http
            .get(format!("{BASE}/__down?bytes=0"))
            .send()
            .await
            .is_ok_and(|r| r.status().is_success());
        if ok {
            let ms = started.elapsed().as_millis().min(u32::MAX as u128) as u32;
            best = Some(best.map_or(ms, |b| b.min(ms)));
        }
    }
    best
}

pub async fn download(http: &reqwest::Client) -> Option<f64> {
    let asked = Instant::now();
    let mut response = http
        .get(format!("{BASE}/__down?bytes={DOWN_BYTES}"))
        .send()
        .await
        .ok()?;
    if !response.status().is_success() {
        return None;
    }
    // Time from the first byte, so the wait for the server (the ping) isn't counted as
    // slow downloading; the first chunk only starts the clock.
    let mut started: Option<Instant> = None;
    let mut got = 0u64;
    while let Ok(Some(chunk)) = response.chunk().await {
        match started {
            None => started = Some(Instant::now()),
            Some(_) => got += chunk.len() as u64,
        }
        if asked.elapsed() > PHASE_LIMIT {
            break;
        }
    }
    let started = started?;
    (got > 0).then(|| mbps(got, started.elapsed()))
}

/// Upload piece sizes: small first so a slow line still finishes one, then doubling so a fast
/// line isn't slowed by the round trip of each piece.
pub fn upload_pieces(total: usize) -> Vec<usize> {
    let mut pieces = Vec::new();
    let (mut size, mut left) = (UP_FIRST_PIECE, total);
    while left > 0 {
        let take = size.min(left);
        pieces.push(take);
        left -= take;
        size = (size * 2).min(UP_MAX_PIECE);
    }
    pieces
}

pub async fn upload(http: &reqwest::Client) -> Option<f64> {
    let started = Instant::now();
    let mut sent = 0u64;
    for piece in upload_pieces(UP_BYTES) {
        let ok = http
            .post(format!("{BASE}/__up"))
            .header(reqwest::header::CONTENT_TYPE, "application/octet-stream")
            .timeout(PHASE_LIMIT)
            .body(vec![0u8; piece])
            .send()
            .await
            .is_ok_and(|r| r.status().is_success());
        if !ok {
            break;
        }
        sent += piece as u64;
        if started.elapsed() > PHASE_LIMIT {
            break;
        }
    }
    (sent > 0).then(|| mbps(sent, started.elapsed()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_bytes_to_megabits() {
        assert_eq!(mbps(12_500_000, Duration::from_secs(1)), 100.0);
        assert_eq!(mbps(1_000_000, Duration::from_millis(3000)), 2.7);
        assert_eq!(mbps(5, Duration::ZERO), 0.0);
    }

    #[test]
    fn uploads_small_pieces_first_then_bigger() {
        let p = upload_pieces(UP_BYTES);
        assert_eq!(p[0], 256 * 1024);
        assert_eq!(p[1], 512 * 1024);
        assert!(p.iter().all(|&x| x <= UP_MAX_PIECE));
        assert_eq!(p.iter().sum::<usize>(), UP_BYTES);
    }
}
