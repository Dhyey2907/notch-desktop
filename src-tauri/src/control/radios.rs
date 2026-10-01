//! Wi-Fi and Bluetooth on/off (Windows.Devices.Radios) and the mobile hotspot
//! (NetworkOperatorTetheringManager). Each switch is the user's click; nothing toggles itself.

use windows::{
    Devices::Radios::{Radio, RadioAccessStatus, RadioKind as WinRadioKind, RadioState},
    Foundation::TypedEventHandler,
    Networking::{
        Connectivity::NetworkInformation,
        NetworkOperators::{NetworkOperatorTetheringManager, TetheringOperationalState},
    },
};

use super::{RadioKind, Radios};
use crate::error::{AppError, AppResult};

fn radios() -> Vec<Radio> {
    Radio::GetRadiosAsync()
        .and_then(|op| op.get())
        .map(|list| list.into_iter().collect())
        .unwrap_or_default()
}

fn find(kind: WinRadioKind) -> Option<Radio> {
    radios().into_iter().find(|r| r.Kind().is_ok_and(|k| k == kind))
}

fn tethering() -> Option<NetworkOperatorTetheringManager> {
    let profile = NetworkInformation::GetInternetConnectionProfile().ok()?;
    NetworkOperatorTetheringManager::CreateFromConnectionProfile(&profile).ok()
}

pub fn read() -> Radios {
    let state = |kind| {
        find(kind)
            .and_then(|r| r.State().ok())
            .map(|s| s == RadioState::On)
    };
    Radios {
        wifi: state(WinRadioKind::WiFi),
        bluetooth: state(WinRadioKind::Bluetooth),
        hotspot: tethering()
            .and_then(|t| t.TetheringOperationalState().ok())
            .map(|s| s == TetheringOperationalState::On),
    }
}

pub fn set(kind: RadioKind, on: bool) -> AppResult<()> {
    let unsupported = || AppError::InvalidInput("This PC doesn't let apps switch that.".into());
    if kind == RadioKind::Hotspot {
        let manager = tethering().ok_or_else(|| {
            AppError::InvalidInput("The hotspot needs an internet connection to share.".into())
        })?;
        let result = if on {
            manager.StartTetheringAsync().and_then(|op| op.get())
        } else {
            manager.StopTetheringAsync().and_then(|op| op.get())
        }
        .map_err(|_| unsupported())?;
        let status = result.Status().map_err(|_| unsupported())?;
        // 0 = Success.
        return if status.0 == 0 {
            Ok(())
        } else {
            Err(AppError::InvalidInput(
                "Windows couldn't switch the hotspot.".into(),
            ))
        };
    }
    let access = Radio::RequestAccessAsync()
        .and_then(|op| op.get())
        .map_err(|_| unsupported())?;
    if access != RadioAccessStatus::Allowed {
        return Err(AppError::InvalidInput(
            "Windows doesn't allow apps to switch radios. Turn on Settings → Privacy & security → Radios."
                .into(),
        ));
    }
    let radio = find(match kind {
        RadioKind::Wifi => WinRadioKind::WiFi,
        _ => WinRadioKind::Bluetooth,
    })
    .ok_or_else(unsupported)?;
    let status = radio
        .SetStateAsync(if on { RadioState::On } else { RadioState::Off })
        .and_then(|op| op.get())
        .map_err(|_| unsupported())?;
    if status == RadioAccessStatus::Allowed {
        Ok(())
    } else {
        Err(unsupported())
    }
}

/// Call `changed` when any radio turns on or off (e.g. from Windows' own quick settings).
pub fn watch(changed: impl Fn() + Clone + Send + Sync + 'static) -> Vec<(Radio, i64)> {
    radios()
        .into_iter()
        .filter_map(|radio| {
            let changed = changed.clone();
            let token = radio
                .StateChanged(&TypedEventHandler::new(move |_, _| {
                    changed();
                    Ok(())
                }))
                .ok()?;
            Some((radio, token))
        })
        .collect()
}
