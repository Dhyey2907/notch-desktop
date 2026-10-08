//! PC lights following the notch (through OpenRGB) and the pet's walk along the taskbar.

use std::{
    sync::{
        atomic::{AtomicU32, Ordering},
        Arc, Mutex,
    },
    time::{Duration, Instant},
};

use tauri::{AppHandle, Manager, PhysicalPosition, PhysicalSize, WebviewUrl, WebviewWindowBuilder};

use super::{openrgb::Client, parse_hex, shade, RgbStatus};
use crate::{events::AppEvent, settings::FunSettings, state::AppState};

const DEFAULT_PORT: u16 = 6742;
/// About 15 updates a second while music plays.
const RGB_TICK: Duration = Duration::from_millis(66);
const RECONNECT_EVERY: Duration = Duration::from_secs(10);
/// White-blue, the notch's own accent when none is chosen.
const DEFAULT_ACCENT: (u8, u8, u8) = (138, 180, 255);
const RESEND_EVERY: Duration = Duration::from_secs(2);
const BEAT_FLASH: Duration = Duration::from_millis(120);

pub const PET_WINDOW: &str = "petwalk";
/// Away this long and the pet goes for a walk.
const PET_AFTER_MS: u64 = 2 * 60_000;
const PET_TICK: Duration = Duration::from_secs(3);
/// While the pet is out, how often to look for a click.
const PET_WATCH: Duration = Duration::from_millis(80);
/// Steady typing for this long also sends it away.
const PET_BUSY_FOR: Duration = Duration::from_secs(15);
/// The jump and the run off the screen (matches the pet window's animation).
const PET_LEAVE: Duration = Duration::from_millis(1700);
/// Tells the pet window to jump and run off.
pub const PET_LEAVE_EVENT: &str = "pet-leave";
/// Logical pixels; scaled for the screen.
const PET_HEIGHT: f64 = 80.0;

#[derive(Default)]
struct Inner {
    /// The album art color while music plays (set by the island).
    music: Option<String>,
    beat_at: Option<Instant>,
    status: Option<RgbStatus>,
}

#[derive(Clone)]
pub struct FunService {
    app: AppHandle,
    inner: Arc<Mutex<Inner>>,
    /// Music loudness 0..1 as f32 bits.
    level: Arc<AtomicU32>,
}

impl FunService {
    pub fn new(app: AppHandle) -> Self {
        Self {
            app,
            inner: Arc::default(),
            level: Arc::default(),
        }
    }

    fn settings(&self) -> (FunSettings, String) {
        let s = self.app.state::<AppState>();
        let s = s.settings.lock().unwrap_or_else(|e| e.into_inner());
        (s.fun.clone(), s.appearance.accent.clone())
    }

    /// The island tells us the music's color (`None` when nothing plays).
    pub fn set_music_color(&self, hex: Option<String>) {
        let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        inner.music = hex.filter(|h| parse_hex(h).is_some());
        if inner.music.is_none() {
            self.level.store(0, Ordering::Relaxed);
        }
    }

    pub fn status(&self) -> RgbStatus {
        self.inner
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .status
            .clone()
            .unwrap_or(RgbStatus {
                state: "off".into(),
                devices: 0,
            })
    }

    fn set_status(&self, state: &str, devices: usize) {
        self.inner.lock().unwrap_or_else(|e| e.into_inner()).status = Some(RgbStatus {
            state: state.into(),
            devices: devices as u32,
        });
    }

    pub fn start(&self) {
        // Loudness and beats, for the lights.
        let this = self.clone();
        let mut events = self.app.state::<AppState>().bus.subscribe();
        tauri::async_runtime::spawn(async move {
            loop {
                match events.recv().await {
                    Ok(envelope) => {
                        if let AppEvent::MusicPulse { level, beat } = envelope.event {
                            this.level.store(level.to_bits(), Ordering::Relaxed);
                            if beat {
                                this.inner.lock().unwrap_or_else(|e| e.into_inner()).beat_at =
                                    Some(Instant::now());
                            }
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                    Err(_) => break,
                }
            }
        });

        let this = self.clone();
        std::thread::Builder::new()
            .name("rgb".into())
            .spawn(move || this.rgb_loop())
            .ok();
        // Its own thread: window calls wait on the main thread, and must never tie up the
        // async runtime's workers while they do.
        let this = self.clone();
        std::thread::Builder::new()
            .name("pet".into())
            .spawn(move || this.pet_loop())
            .ok();
    }

    /// The color the lights should have now.
    fn target(&self, accent: &str) -> (u8, u8, u8) {
        let inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        match inner.music.as_deref().and_then(parse_hex) {
            Some(color) => {
                let level = f32::from_bits(self.level.load(Ordering::Relaxed));
                let beat = inner.beat_at.is_some_and(|at| at.elapsed() < BEAT_FLASH);
                shade(color, level, beat)
            }
            None => parse_hex(accent).unwrap_or(DEFAULT_ACCENT),
        }
    }

    fn rgb_loop(&self) {
        let mut client: Option<(Client, u16)> = None;
        let mut last_try: Option<Instant> = None;
        let mut last_color = None;
        let mut last_sent: Option<Instant> = None;
        loop {
            let (fun, accent) = self.settings();
            let port = if fun.rgb_port == 0 {
                DEFAULT_PORT
            } else {
                fun.rgb_port
            };
            if !fun.rgb {
                client = None;
                last_color = None;
                self.set_status("off", 0);
                std::thread::sleep(Duration::from_secs(1));
                continue;
            }
            if client.as_ref().is_some_and(|(_, p)| *p != port) {
                client = None;
            }
            if client.is_none() {
                if last_try.is_some_and(|t| t.elapsed() < RECONNECT_EVERY) {
                    std::thread::sleep(Duration::from_millis(500));
                    continue;
                }
                last_try = Some(Instant::now());
                match Client::connect(port) {
                    Ok(c) => {
                        tracing::info!(devices = c.devices.len(), "connected to OpenRGB");
                        self.set_status("connected", c.devices.len());
                        client = Some((c, port));
                        last_color = None;
                    }
                    Err(_) => {
                        self.set_status("notRunning", 0);
                        continue;
                    }
                }
            }
            let color = self.target(&accent);
            // Resend now and then even when nothing changed: that's how a closed OpenRGB is noticed.
            if last_color != Some(color) || last_sent.is_none_or(|t| t.elapsed() >= RESEND_EVERY) {
                if let Some((c, _)) = client.as_mut() {
                    if c.set_all(color).is_err() {
                        tracing::info!("lost the OpenRGB connection");
                        client = None;
                        self.set_status("notRunning", 0);
                        continue;
                    }
                }
                last_color = Some(color);
                last_sent = Some(Instant::now());
            }
            std::thread::sleep(RGB_TICK);
        }
    }

    /// The pet walks the taskbar while you're away. When you come back and click (or type for
    /// a while), it jumps and runs off the screen.
    fn pet_loop(&self) {
        let mut out = false;
        let mut busy_since: Option<Instant> = None;
        loop {
            let (fun, _) = self.settings();
            #[cfg(windows)]
            let (idle, locked) = (
                crate::system::screentime::probe::idle_ms(),
                crate::system::screentime::probe::locked(),
            );
            #[cfg(not(windows))]
            let (idle, locked) = (0, false);
            // Never over a fullscreen app on any screen, never while something plays (you're
            // watching or listening, not away), and never behind the lock screen.
            let playing = self
                .app
                .state::<crate::system::service::SystemService>()
                .media_playing();
            let allowed = fun.pet_walk && !locked && !playing && !crate::shell::any_fullscreen();
            if !out {
                // Demo mode (recording the website's videos) doesn't wait two minutes.
                let after = if crate::demo() { 8_000 } else { PET_AFTER_MS };
                if allowed && idle >= after {
                    out = self.show_pet();
                    busy_since = None;
                }
                std::thread::sleep(if out { PET_WATCH } else { PET_TICK });
                continue;
            }
            if !allowed {
                self.hide_pet();
                out = false;
                continue;
            }
            busy_since = if idle < 1500 {
                busy_since.or(Some(Instant::now()))
            } else {
                None
            };
            if mouse_clicked() || busy_since.is_some_and(|t| t.elapsed() >= PET_BUSY_FOR) {
                use tauri::Emitter;
                let _ = self.app.emit_to(PET_WINDOW, PET_LEAVE_EVENT, ());
                std::thread::sleep(PET_LEAVE);
                self.hide_pet();
                out = false;
                continue;
            }
            std::thread::sleep(PET_WATCH);
        }
    }

    /// Bring the pet out (opening its window the first time). Returns whether it's showing.
    fn show_pet(&self) -> bool {
        match self.app.get_webview_window(PET_WINDOW) {
            Some(w) => {
                self.place_pet(&w);
                w.show().is_ok()
            }
            None => match self.open_pet() {
                Ok(()) => true,
                Err(e) => {
                    tracing::warn!(error = %e, "could not open the walking pet");
                    false
                }
            },
        }
    }

    fn hide_pet(&self) {
        if let Some(w) = self.app.get_webview_window(PET_WINDOW) {
            let _ = w.hide();
        }
    }

    fn open_pet(&self) -> tauri::Result<()> {
        let w = WebviewWindowBuilder::new(&self.app, PET_WINDOW, WebviewUrl::App("index.html#pet".into()))
            .title("Overline pet")
            .decorations(false)
            .transparent(true)
            .always_on_top(true)
            .skip_taskbar(true)
            .resizable(false)
            .shadow(false)
            .focused(false)
            .focusable(false)
            .visible(false)
            .build()?;
        w.set_ignore_cursor_events(true)?;
        self.place_pet(&w);
        w.show()
    }

    /// Standing on the taskbar of the screen the notch is on.
    fn place_pet(&self, w: &tauri::WebviewWindow) {
        let notch_screen = self
            .app
            .get_webview_window(crate::shell::MAIN_WINDOW)
            .and_then(|main| main.current_monitor().ok().flatten());
        let Some(monitor) = notch_screen.or_else(|| w.primary_monitor().ok().flatten()) else {
            return;
        };
        let area = monitor.work_area();
        let (y, height) = pet_strip(
            (area.position.y, area.size.height),
            (PET_HEIGHT * monitor.scale_factor()).round() as u32,
        );
        let _ = w.set_size(PhysicalSize::new(area.size.width, height));
        let _ = w.set_position(PhysicalPosition::new(area.position.x, y));
    }
}

/// Where the pet walks: a strip whose bottom is the taskbar's top edge (the bottom of the work
/// area), so the pet stands on the taskbar. `(top, height)` in physical pixels.
fn pet_strip(work: (i32, u32), height: u32) -> (i32, u32) {
    (work.0 + work.1 as i32 - height as i32, height)
}

/// A mouse button is down (or was pressed since the last look).
fn mouse_clicked() -> bool {
    #[cfg(windows)]
    {
        use windows::Win32::UI::Input::KeyboardAndMouse::{
            GetAsyncKeyState, VK_LBUTTON, VK_MBUTTON, VK_RBUTTON,
        };
        // SAFETY: reads the asynchronous key state; no pointers involved.
        [VK_LBUTTON, VK_RBUTTON, VK_MBUTTON]
            .iter()
            .any(|k| unsafe { GetAsyncKeyState(i32::from(k.0)) } as u16 & 0x8001 != 0)
    }
    #[cfg(not(windows))]
    false
}

#[cfg(test)]
mod pet_tests {
    use super::pet_strip;

    #[test]
    fn stands_on_the_taskbar_edge() {
        // 1080p screen with a 48 px taskbar: the strip ends exactly where the taskbar starts.
        assert_eq!(pet_strip((0, 1032), 80), (952, 80));
        // A second screen below the first.
        assert_eq!(pet_strip((1080, 1368), 120), (2328, 120));
    }
}
