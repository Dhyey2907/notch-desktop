//! Starts the watchers and turns their reports into app events.

use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    time::Duration,
};

use tauri::{AppHandle, Manager};

use super::{BluetoothDevice, Dashboard, Desktops, SoundState, SystemStats, Weather, WeatherStatus};
use crate::{
    error::{AppError, AppResult},
    events::AppEvent,
    state::AppState,
};

/// Weather refresh: every half hour, sooner after a failure.
const WEATHER_EVERY: Duration = Duration::from_secs(30 * 60);
const WEATHER_RETRY: Duration = Duration::from_secs(5 * 60);

/// Last desktops sent, so a front-window change only announces real switches.
static LAST_DESKTOPS: Mutex<Option<Desktops>> = Mutex::new(None);

/// Announce the virtual desktops if they changed (called on registry and front-window changes).
pub fn desktops_maybe_changed(app: &AppHandle) {
    #[cfg(windows)]
    {
        let now = super::desktops::read();
        let mut last = LAST_DESKTOPS.lock().unwrap_or_else(|e| e.into_inner());
        if last.as_ref() != Some(&now) {
            *last = Some(now.clone());
            app.state::<AppState>()
                .bus
                .emit("system", AppEvent::DesktopsChanged(now));
        }
    }
    #[cfg(not(windows))]
    let _ = app;
}

#[derive(Default)]
struct Latest {
    sound: Option<SoundState>,
    brightness: Option<u8>,
    weather: Option<Weather>,
}

#[derive(Clone)]
pub struct SystemService {
    app: AppHandle,
    latest: Arc<Mutex<Latest>>,
    #[cfg(windows)]
    audio: Arc<Mutex<Option<super::audio::Audio>>>,
    #[cfg(windows)]
    bluetooth: Arc<Mutex<Option<super::bluetooth::Bluetooth>>>,
    /// Media is playing (counts as screen time without input).
    media_playing: Arc<AtomicBool>,
    /// Wakes the weather loop for a refresh now.
    weather_now: Arc<tokio::sync::Notify>,
}

impl SystemService {
    /// Something is playing (music or video) — you may be watching even without touching
    /// the keyboard.
    pub fn media_playing(&self) -> bool {
        self.media_playing.load(Ordering::Relaxed)
    }

    pub fn new(app: AppHandle) -> Self {
        Self {
            app,
            latest: Arc::default(),
            #[cfg(windows)]
            audio: Arc::default(),
            #[cfg(windows)]
            bluetooth: Arc::default(),
            media_playing: Arc::default(),
            weather_now: Arc::default(),
        }
    }

    pub fn start(&self) {
        self.start_dashboard();
        #[cfg(windows)]
        {
            use super::audio::Cause;
            let (app, latest) = (self.app.clone(), self.latest.clone());
            let audio = super::audio::start(move |state, cause| {
                latest.lock().unwrap_or_else(|e| e.into_inner()).sound = Some(state.clone());
                if cause != Cause::Initial {
                    let switched = cause == Cause::DeviceSwitched;
                    app.state::<AppState>()
                        .bus
                        .emit("system", AppEvent::SoundChanged { state, switched });
                }
            });
            *self.audio.lock().unwrap_or_else(|e| e.into_inner()) = Some(audio);

            let (app, latest) = (self.app.clone(), self.latest.clone());
            super::brightness::start(move |level, initial| {
                let changed = {
                    let mut latest = latest.lock().unwrap_or_else(|e| e.into_inner());
                    let changed = latest.brightness != Some(level);
                    latest.brightness = Some(level);
                    changed
                };
                if changed && !initial {
                    app.state::<AppState>()
                        .bus
                        .emit("system", AppEvent::BrightnessChanged { level });
                }
            });

            let app = self.app.clone();
            let bluetooth = super::bluetooth::start(move |device| {
                app.state::<AppState>()
                    .bus
                    .emit("system", AppEvent::BluetoothChanged(device));
            });
            *self.bluetooth.lock().unwrap_or_else(|e| e.into_inner()) = Some(bluetooth);
        }
    }

    /// Screen time ticker, desktop watcher and weather loop.
    fn start_dashboard(&self) {
        // Media playing counts as active use (watching a video without touching anything).
        let playing = self.media_playing.clone();
        let mut events = self.app.state::<AppState>().bus.subscribe();
        tauri::async_runtime::spawn(async move {
            loop {
                match events.recv().await {
                    Ok(envelope) => {
                        if let AppEvent::MediaStateChanged { state } = &envelope.event {
                            let on = state
                                .as_ref()
                                .is_some_and(|s| s.status == crate::media::PlaybackStatus::Playing);
                            playing.store(on, Ordering::Relaxed);
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                    Err(_) => break,
                }
            }
        });

        let this = self.clone();
        tauri::async_runtime::spawn(async move {
            let mut tick = tokio::time::interval(Duration::from_secs(60));
            tick.tick().await;
            loop {
                tick.tick().await;
                #[cfg(windows)]
                let active = super::screentime::counts(
                    super::screentime::probe::idle_ms(),
                    super::screentime::probe::locked(),
                    this.media_playing.load(Ordering::Relaxed),
                );
                #[cfg(not(windows))]
                let active = false;
                if !active {
                    continue;
                }
                let now = chrono::Local::now();
                let state = this.app.state::<AppState>();
                let today = state.db.with(|conn| {
                    super::screentime::add_minute(conn, now)?;
                    super::screentime::today(conn, now)
                });
                match today {
                    Ok(today) => state.bus.emit("system", AppEvent::ScreenTimeChanged(today)),
                    Err(error) => tracing::warn!(kind = error.kind(), "could not record screen time"),
                }
            }
        });

        #[cfg(windows)]
        {
            let app = self.app.clone();
            super::desktops::watch(move || desktops_maybe_changed(&app));
        }

        let this = self.clone();
        tauri::async_runtime::spawn(async move {
            let http = reqwest::Client::builder()
                .timeout(Duration::from_secs(15))
                .user_agent("Notch-Desktop")
                .build()
                .expect("static client config");
            loop {
                let located = tauri::async_runtime::spawn_blocking(super::weather::locate)
                    .await
                    .unwrap_or(super::weather::Located::Failed);
                let weather = match located {
                    super::weather::Located::At(lat, lon) => super::weather::fetch(&http, lat, lon)
                        .await
                        .unwrap_or_else(|| Weather::with_status(WeatherStatus::Unavailable)),
                    super::weather::Located::NoAccess => Weather::with_status(WeatherStatus::NoAccess),
                    super::weather::Located::Failed => Weather::with_status(WeatherStatus::Unavailable),
                };
                let wait = if weather.status == WeatherStatus::Ready {
                    WEATHER_EVERY
                } else {
                    WEATHER_RETRY
                };
                this.latest.lock().unwrap_or_else(|e| e.into_inner()).weather = Some(weather.clone());
                this.app
                    .state::<AppState>()
                    .bus
                    .emit("system", AppEvent::WeatherChanged(weather));
                tokio::select! {
                    _ = tokio::time::sleep(wait) => {}
                    _ = this.weather_now.notified() => {}
                }
            }
        });
    }

    pub fn dashboard(&self) -> AppResult<Dashboard> {
        let latest = self.latest.lock().unwrap_or_else(|e| e.into_inner());
        let screen_time = self
            .app
            .state::<AppState>()
            .db
            .with(|conn| super::screentime::today(conn, chrono::Local::now()))?;
        Ok(Dashboard {
            sound: latest.sound.clone(),
            brightness: latest.brightness,
            screen_time,
            desktops: self.desktops(),
            weather: latest
                .weather
                .clone()
                .unwrap_or_else(|| Weather::with_status(WeatherStatus::Loading)),
        })
    }

    pub fn stats(&self) -> SystemStats {
        #[cfg(windows)]
        return super::stats::read();
        #[cfg(not(windows))]
        SystemStats {
            cpu: None,
            ram: None,
            battery: None,
            charging: false,
        }
    }

    pub fn desktops(&self) -> Desktops {
        #[cfg(windows)]
        return super::desktops::read();
        #[cfg(not(windows))]
        Desktops::default()
    }

    /// Switch Windows virtual desktop (blocks while the shortcut is pressed).
    pub fn switch_desktop(&self, index: u32) -> AppResult<()> {
        #[cfg(windows)]
        if super::desktops::switch_to(index) {
            return Ok(());
        }
        let _ = index;
        Err(AppError::InvalidInput("Couldn't switch desktops".into()))
    }

    pub fn set_brightness(&self, level: u8) -> AppResult<()> {
        if self
            .latest
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .brightness
            .is_none()
        {
            return Err(AppError::InvalidInput(
                "This screen's brightness can't be changed here".into(),
            ));
        }
        #[cfg(windows)]
        if super::brightness_set::set(level) {
            return Ok(());
        }
        let _ = level;
        Err(AppError::Platform("brightness control unavailable".into()))
    }

    pub fn refresh_weather(&self) {
        self.weather_now.notify_one();
    }

    pub fn sound(&self) -> Option<SoundState> {
        self.latest
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .sound
            .clone()
    }

    /// Connected Bluetooth devices, with battery where reported (blocks briefly).
    pub fn bluetooth(&self) -> Vec<BluetoothDevice> {
        #[cfg(windows)]
        if let Some(bluetooth) = self.bluetooth.lock().unwrap_or_else(|e| e.into_inner()).clone() {
            return bluetooth.connected();
        }
        Vec::new()
    }

    pub fn set_volume(&self, level: f32) {
        #[cfg(windows)]
        if let Some(audio) = self.audio.lock().unwrap_or_else(|e| e.into_inner()).as_ref() {
            audio.set_level(level);
        }
        #[cfg(not(windows))]
        let _ = level;
    }

    pub fn toggle_mute(&self) {
        #[cfg(windows)]
        if let Some(audio) = self.audio.lock().unwrap_or_else(|e| e.into_inner()).as_ref() {
            audio.toggle_mute();
        }
    }
}
