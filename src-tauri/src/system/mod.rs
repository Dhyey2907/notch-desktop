//! System readouts for the notch's HUD: speaker volume, laptop screen brightness, and
//! Bluetooth devices connecting. All event-driven; nothing here is logged beyond failures.

#[cfg(windows)]
mod audio;
#[cfg(windows)]
mod bluetooth;
#[cfg(windows)]
mod brightness;
pub mod service;

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
