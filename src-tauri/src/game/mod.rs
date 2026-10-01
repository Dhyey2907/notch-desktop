//! Game mode: while a game you've marked plays fullscreen, the notch shows a slim HUD — FPS
//! (from Intel's open-source PresentMon, for that game only), GPU temperature and load (NVIDIA's
//! NVML), CPU, RAM, ping and the time — and holds other cards until you're done.

#[cfg(windows)]
mod nvml;
#[cfg(windows)]
mod ping;
pub mod service;

use std::collections::{HashMap, VecDeque};

use serde::Serialize;
use ts_rs::TS;

/// PresentMon's console build, pinned: Notch downloads exactly this file and checks its hash.
pub const PRESENTMON_URL: &str =
    "https://github.com/GameTechDev/PresentMon/releases/download/v2.6.0/PresentMon-2.6.0-x64.exe";
pub const PRESENTMON_SHA256: &str = "b2a706bc6ad475749e3b7e3409263aa1e6906d45bdcf993f6dbc0f660188f1af";
pub const PRESENTMON_FILE: &str = "PresentMon-2.6.0-x64.exe";

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct GameHud {
    /// Game name, e.g. "Valorant".
    pub app: String,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fps: Option<u32>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gpu_temp: Option<u32>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gpu_load: Option<u32>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cpu: Option<u8>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ram: Option<u8>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ping_ms: Option<u32>,
}

/// Apps that go fullscreen but aren't games: never asked about.
const NOT_GAMES: &[&str] = &[
    "msedge.exe",
    "chrome.exe",
    "firefox.exe",
    "brave.exe",
    "opera.exe",
    "vivaldi.exe",
    "arc.exe",
    "explorer.exe",
    "vlc.exe",
    "mpv.exe",
    "mpc-hc64.exe",
    "potplayermini64.exe",
    "wmplayer.exe",
    "video.ui.exe",
    "microsoft.media.player.exe",
    "photos.exe",
    "powerpnt.exe",
    "winword.exe",
    "excel.exe",
    "code.exe",
    "windowsterminal.exe",
    "applicationframehost.exe",
    "spotify.exe",
    "netflix.exe",
    "claude.exe",
    "notch-desktop.exe",
    "teams.exe",
    "ms-teams.exe",
    "zoom.exe",
    "obs64.exe",
    "acrobat.exe",
    "sumatrapdf.exe",
    "lockapp.exe",
    "searchhost.exe",
];

pub fn never_a_game(exe_key: &str) -> bool {
    NOT_GAMES.contains(&exe_key)
}

/// Column positions in PresentMon's CSV header.
pub struct Columns {
    pid: usize,
    between: usize,
}

impl Columns {
    pub fn from_header(header: &str) -> Option<Self> {
        let names: Vec<&str> = header.split(',').map(str::trim).collect();
        Some(Self {
            pid: names.iter().position(|n| *n == "ProcessID")?,
            between: names.iter().position(|n| *n == "MsBetweenPresents")?,
        })
    }

    /// (process id, milliseconds since the previous frame) from one CSV row.
    pub fn frame(&self, line: &str) -> Option<(u32, f64)> {
        let fields: Vec<&str> = line.split(',').collect();
        let pid = fields.get(self.pid)?.trim().parse().ok()?;
        let ms: f64 = fields.get(self.between)?.trim().parse().ok()?;
        (ms.is_finite() && ms > 0.0 && ms < 5_000.0).then_some((pid, ms))
    }
}

/// FPS over the last second of frames (per swap chain owner, the busiest one wins).
#[derive(Default)]
pub struct FpsMeter {
    frames: HashMap<u32, VecDeque<f64>>,
}

impl FpsMeter {
    pub fn add(&mut self, pid: u32, ms: f64) {
        let q = self.frames.entry(pid).or_default();
        q.push_back(ms);
        // Keep about one second of frames.
        while q.iter().sum::<f64>() > 1_000.0 && q.len() > 1 {
            q.pop_front();
        }
    }

    pub fn fps(&self) -> Option<u32> {
        self.frames
            .values()
            .filter(|q| !q.is_empty())
            .map(|q| (1_000.0 / (q.iter().sum::<f64>() / q.len() as f64)).round() as u32)
            .max()
    }

    pub fn clear(&mut self) {
        self.frames.clear();
    }
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::Digest;
    sha2::Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const HEADER: &str = "Application,ProcessID,SwapChainAddress,PresentRuntime,SyncInterval,PresentFlags,AllowsTearing,PresentMode,TimeInMs,MsBetweenSimulationStart,MsBetweenPresents,MsBetweenDisplayChange";

    #[test]
    fn reads_frames_from_presentmon_csv() {
        let cols = Columns::from_header(HEADER).unwrap();
        let row = "game.exe,4242,0x1,DXGI,0,0,1,Hardware: Independent Flip,1.0,NA,6.94,6.94";
        assert_eq!(cols.frame(row), Some((4242, 6.94)));
        assert_eq!(cols.frame("game.exe,4242,0x1,DXGI,0,0,1,x,1.0,NA,NA,NA"), None);
        assert!(Columns::from_header("Application,Nope").is_none());
    }

    #[test]
    fn measures_fps_over_the_last_second() {
        let mut meter = FpsMeter::default();
        assert_eq!(meter.fps(), None);
        for _ in 0..300 {
            meter.add(1, 1000.0 / 144.0);
        }
        assert_eq!(meter.fps(), Some(144));
        // A slow background app doesn't drag the game's number down.
        for _ in 0..10 {
            meter.add(2, 100.0);
        }
        assert_eq!(meter.fps(), Some(144));
        // Frame times change: the old second drops out.
        for _ in 0..120 {
            meter.add(1, 1000.0 / 60.0);
        }
        assert_eq!(meter.fps(), Some(60));
    }

    #[test]
    fn knows_common_non_games() {
        assert!(never_a_game("chrome.exe"));
        assert!(never_a_game("vlc.exe"));
        assert!(!never_a_game("valorant-win64-shipping.exe"));
    }

    #[test]
    fn hashes_files() {
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }
}
