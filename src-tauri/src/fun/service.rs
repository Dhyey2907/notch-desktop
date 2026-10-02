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
/// Logical pixels; scaled for the screen.
const PET_HEIGHT: f64 = 40.0;

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
        let this = self.clone();
        tauri::async_runtime::spawn(async move {
            loop {
                tokio::time::sleep(PET_TICK).await;
                this.pet_tick();
            }
        });
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

    /// Show the walking pet while you're away; hide it the moment you're back.
    fn pet_tick(&self) {
        let (fun, _) = self.settings();
        #[cfg(windows)]
        let (idle, locked) = (
            crate::system::screentime::probe::idle_ms(),
            crate::system::screentime::probe::locked(),
        );
        #[cfg(not(windows))]
        let (idle, locked) = (0, false);
        // Not over a fullscreen video or game on the main screen, and not behind the lock screen.
        let want = fun.pet_walk && idle >= PET_AFTER_MS && !locked && !crate::shell::is_fullscreen("main");
        let window = self.app.get_webview_window(PET_WINDOW);
        match (want, window) {
            (true, Some(w)) => {
                if !w.is_visible().unwrap_or(false) {
                    self.place_pet(&w);
                    let _ = w.show();
                }
            }
            (true, None) => {
                if let Err(e) = self.open_pet() {
                    tracing::warn!(error = %e, "could not open the walking pet");
                }
            }
            (false, Some(w)) => {
                if w.is_visible().unwrap_or(false) {
                    let _ = w.hide();
                }
            }
            (false, None) => {}
        }
    }

    fn open_pet(&self) -> tauri::Result<()> {
        let w = WebviewWindowBuilder::new(&self.app, PET_WINDOW, WebviewUrl::App("index.html#pet".into()))
            .title("Notch pet")
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

    /// A strip just above the taskbar, across the main screen.
    fn place_pet(&self, w: &tauri::WebviewWindow) {
        let Ok(Some(monitor)) = w.primary_monitor() else {
            return;
        };
        let area = monitor.work_area();
        let height = (PET_HEIGHT * monitor.scale_factor()).round() as u32;
        let _ = w.set_size(PhysicalSize::new(area.size.width, height));
        let _ = w.set_position(PhysicalPosition::new(
            area.position.x,
            area.position.y + area.size.height as i32 - height as i32,
        ));
    }
}
