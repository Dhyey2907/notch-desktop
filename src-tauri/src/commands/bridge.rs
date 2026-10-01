use tauri::State;

use crate::{
    bridge::{service::BridgeService, BridgeState},
    error::AppResult,
};

#[tauri::command]
pub fn bridge_state(bridge: State<'_, BridgeService>) -> Option<BridgeState> {
    bridge.state()
}

/// Offer files to the phone (none: just receive); returns the QR code and address.
#[tauri::command]
pub fn bridge_start(paths: Vec<String>, bridge: State<'_, BridgeService>) -> AppResult<BridgeState> {
    bridge.start(paths)
}

#[tauri::command]
pub fn bridge_stop(bridge: State<'_, BridgeService>) {
    bridge.stop();
}
