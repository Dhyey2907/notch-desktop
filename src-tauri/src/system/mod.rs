//! System readouts for the notch's HUD: speaker volume, laptop screen brightness, and
//! Bluetooth devices connecting. All event-driven; nothing here is logged beyond failures.

#[cfg(windows)]
mod audio;
#[cfg(windows)]
mod bluetooth;
#[cfg(windows)]
mod brightness;
#[cfg(windows)]
mod brightness_set;
#[cfg(windows)]
mod desktops;
pub mod screentime;
pub mod service;
#[cfg(windows)]
mod stats;
mod weather;

use serde::Serialize;
use ts_rs::TS;

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SoundState {
    /// 0.0 – 1.0.
    pub level: f32,
    pub muted: bool,
    /// Output device, e.g. "Speakers" or "Galaxy Buds2 Pro".
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub device: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct BluetoothDevice {
    pub name: String,
    pub connected: bool,
    /// Percent, when the device reports it.
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub battery: Option<u8>,
}

/// CPU, memory and battery, for the Home dashboard.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SystemStats {
    /// Percent busy since the previous reading (none on the first one).
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cpu: Option<u8>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ram: Option<u8>,
    /// None on a PC without a battery.
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub battery: Option<u8>,
    pub charging: bool,
}

/// Active minutes today, total and per local hour (24 entries).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ScreenTime {
    pub minutes: u32,
    pub hours: Vec<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct VirtualDesktop {
    pub index: u32,
    pub name: String,
    pub current: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Desktops {
    pub desktops: Vec<VirtualDesktop>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum WeatherStatus {
    Loading,
    Ready,
    /// Windows location is off or not allowed for desktop apps.
    NoAccess,
    /// Offline, or the weather service didn't answer.
    Unavailable,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Weather {
    pub status: WeatherStatus,
    /// Degrees Celsius.
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<i32>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub condition: Option<String>,
    /// sun, moon, cloud-sun, cloud-moon, cloud, fog, drizzle, rain, snow, storm.
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    /// Town or city, e.g. "Ahmedabad".
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub place: Option<String>,
}

impl Weather {
    pub fn with_status(status: WeatherStatus) -> Self {
        Self {
            status,
            temperature: None,
            condition: None,
            icon: None,
            place: None,
        }
    }
}

/// Everything the Home dashboard shows, in one read.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Dashboard {
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sound: Option<SoundState>,
    /// Laptop screen brightness, percent (none without a built-in screen).
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub brightness: Option<u8>,
    pub screen_time: ScreenTime,
    pub desktops: Desktops,
    pub weather: Weather,
}
