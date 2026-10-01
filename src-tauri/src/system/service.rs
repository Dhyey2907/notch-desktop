//! Starts the watchers and turns their reports into app events.

use std::sync::{Arc, Mutex};

use tauri::{AppHandle, Manager};

use super::{BluetoothDevice, SoundState};
use crate::{events::AppEvent, state::AppState};

#[derive(Default)]
struct Latest {
    sound: Option<SoundState>,
    brightness: Option<u8>,
}

#[derive(Clone)]
pub struct SystemService {
    app: AppHandle,
    latest: Arc<Mutex<Latest>>,
    #[cfg(windows)]
    audio: Arc<Mutex<Option<super::audio::Audio>>>,
    #[cfg(windows)]
    bluetooth: Arc<Mutex<Option<super::bluetooth::Bluetooth>>>,
}

impl SystemService {
    pub fn new(app: AppHandle) -> Self {
        Self {
            app,
            latest: Arc::default(),
            #[cfg(windows)]
            audio: Arc::default(),
            #[cfg(windows)]
            bluetooth: Arc::default(),
        }
    }

    pub fn start(&self) {
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
