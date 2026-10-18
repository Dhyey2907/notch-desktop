//! Display quick toggles for Control: night light, HDR, and the screen's refresh rate.
//! Night light has no public API: Windows keeps its state in a small registry blob, which is
//! flipped the same way Settings does; if the blob isn't the expected shape, the button opens
//! Windows' Night light settings instead. HDR and refresh rate use the display APIs.

use serde::Serialize;
use ts_rs::TS;

#[derive(Debug, Clone, Default, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct DisplayToggles {
    /// None: couldn't read it (the button opens Settings).
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub night_light: Option<bool>,
    /// None: no HDR-capable screen.
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hdr: Option<bool>,
    /// The main screen's refresh rate now, and the rates it offers at this resolution.
    pub hz: u32,
    pub rates: Vec<u32>,
}

/// Night light's blob: byte 18 is 0x15 when on, 0x13 when off, and "on" carries two extra
/// bytes (0x10 0x00) at 23. Returns the flipped blob, or None if it isn't the shape we know.
pub fn flip_night_light(blob: &[u8], on: bool) -> Option<Vec<u8>> {
    if blob.len() < 26 || !matches!(blob[18], 0x13 | 0x15) {
        return None;
    }
    let is_on = blob[18] == 0x15;
    if is_on == on {
        return Some(blob.to_vec());
    }
    let mut out = blob.to_vec();
    // Bump the change counter (bytes 10–14) so Windows notices.
    for b in out.iter_mut().take(15).skip(10) {
        if *b != 0xFF {
            *b += 1;
            break;
        }
    }
    if on {
        out[18] = 0x15;
        out.splice(23..23, [0x10, 0x00]);
    } else {
        out[18] = 0x13;
        if out.get(23..25) == Some(&[0x10, 0x00]) {
            out.drain(23..25);
        }
    }
    Some(out)
}

pub fn night_light_state(blob: &[u8]) -> Option<bool> {
    (blob.len() >= 26).then_some(())?;
    match blob[18] {
        0x15 => Some(true),
        0x13 => Some(false),
        _ => None,
    }
}

#[cfg(windows)]
pub mod win {
    use windows::{
        core::{HSTRING, PCWSTR},
        Win32::{
            Devices::Display::{
                DisplayConfigGetDeviceInfo, DisplayConfigSetDeviceInfo, GetDisplayConfigBufferSizes,
                QueryDisplayConfig, DISPLAYCONFIG_DEVICE_INFO_GET_ADVANCED_COLOR_INFO,
                DISPLAYCONFIG_DEVICE_INFO_SET_ADVANCED_COLOR_STATE, DISPLAYCONFIG_GET_ADVANCED_COLOR_INFO,
                DISPLAYCONFIG_MODE_INFO, DISPLAYCONFIG_PATH_INFO, DISPLAYCONFIG_SET_ADVANCED_COLOR_STATE,
                QDC_ONLY_ACTIVE_PATHS,
            },
            Graphics::Gdi::{
                ChangeDisplaySettingsExW, EnumDisplaySettingsW, CDS_UPDATEREGISTRY, DEVMODEW,
                DISP_CHANGE_SUCCESSFUL, DM_DISPLAYFREQUENCY, ENUM_CURRENT_SETTINGS,
                ENUM_DISPLAY_SETTINGS_MODE,
            },
            System::Registry::{
                RegCloseKey, RegOpenKeyExW, RegQueryValueExW, RegSetValueExW, HKEY, HKEY_CURRENT_USER,
                KEY_READ, KEY_WRITE, REG_BINARY,
            },
        },
    };

    use super::DisplayToggles;
    use crate::error::{AppError, AppResult};

    const NIGHT_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\CloudStore\Store\DefaultAccount\Current\default$windows.data.bluelightreduction.bluelightreductionstate\windows.data.bluelightreduction.bluelightreductionstate";

    fn night_blob() -> Option<Vec<u8>> {
        // SAFETY: registry reads into local buffers; the key is closed before returning.
        unsafe {
            let mut key = HKEY::default();
            if RegOpenKeyExW(
                HKEY_CURRENT_USER,
                &HSTRING::from(NIGHT_KEY),
                None,
                KEY_READ,
                &mut key,
            )
            .is_err()
            {
                return None;
            }
            let mut size = 0u32;
            let name = HSTRING::from("Data");
            let _ = RegQueryValueExW(key, &name, None, None, None, Some(&mut size));
            let mut buf = vec![0u8; size as usize];
            let ok =
                RegQueryValueExW(key, &name, None, None, Some(buf.as_mut_ptr()), Some(&mut size)).is_ok();
            let _ = RegCloseKey(key);
            ok.then(|| {
                buf.truncate(size as usize);
                buf
            })
        }
    }

    pub fn set_night_light(on: bool) -> AppResult<()> {
        let blob = night_blob().ok_or_else(|| AppError::InvalidInput("night light settings".into()))?;
        let next = super::flip_night_light(&blob, on)
            .ok_or_else(|| AppError::InvalidInput("night light settings".into()))?;
        // SAFETY: writes the flipped blob back to the same value; the key is closed after.
        unsafe {
            let mut key = HKEY::default();
            RegOpenKeyExW(
                HKEY_CURRENT_USER,
                &HSTRING::from(NIGHT_KEY),
                None,
                KEY_WRITE,
                &mut key,
            )
            .ok()
            .map_err(|e| AppError::Platform(e.to_string()))?;
            let result = RegSetValueExW(key, &HSTRING::from("Data"), None, REG_BINARY, Some(&next));
            let _ = RegCloseKey(key);
            result.ok().map_err(|e| AppError::Platform(e.to_string()))
        }
    }

    fn paths() -> Vec<DISPLAYCONFIG_PATH_INFO> {
        // SAFETY: sizes first, then the buffers of those sizes.
        unsafe {
            let (mut np, mut nm) = (0u32, 0u32);
            if GetDisplayConfigBufferSizes(QDC_ONLY_ACTIVE_PATHS, &mut np, &mut nm).is_err() {
                return Vec::new();
            }
            let mut p = vec![DISPLAYCONFIG_PATH_INFO::default(); np as usize];
            let mut m = vec![DISPLAYCONFIG_MODE_INFO::default(); nm as usize];
            if QueryDisplayConfig(
                QDC_ONLY_ACTIVE_PATHS,
                &mut np,
                p.as_mut_ptr(),
                &mut nm,
                m.as_mut_ptr(),
                None,
            )
            .is_err()
            {
                return Vec::new();
            }
            p.truncate(np as usize);
            p
        }
    }

    /// HDR on the screens that support it: Some(on) if any screen can do HDR.
    fn hdr_state() -> Option<bool> {
        let mut any = None;
        for path in paths() {
            let mut info = DISPLAYCONFIG_GET_ADVANCED_COLOR_INFO::default();
            info.header.r#type = DISPLAYCONFIG_DEVICE_INFO_GET_ADVANCED_COLOR_INFO;
            info.header.size = std::mem::size_of::<DISPLAYCONFIG_GET_ADVANCED_COLOR_INFO>() as u32;
            info.header.adapterId = path.targetInfo.adapterId;
            info.header.id = path.targetInfo.id;
            // SAFETY: a correctly sized query packet for one active target.
            if unsafe { DisplayConfigGetDeviceInfo(&mut info.header) } != 0 {
                continue;
            }
            // Bit 0: supported, bit 1: enabled.
            let bits = unsafe { info.Anonymous.value };
            if bits & 1 != 0 {
                any = Some(any.unwrap_or(false) || bits & 2 != 0);
            }
        }
        any
    }

    pub fn set_hdr(on: bool) -> AppResult<()> {
        let mut done = false;
        for path in paths() {
            let mut set = DISPLAYCONFIG_SET_ADVANCED_COLOR_STATE::default();
            set.header.r#type = DISPLAYCONFIG_DEVICE_INFO_SET_ADVANCED_COLOR_STATE;
            set.header.size = std::mem::size_of::<DISPLAYCONFIG_SET_ADVANCED_COLOR_STATE>() as u32;
            set.header.adapterId = path.targetInfo.adapterId;
            set.header.id = path.targetInfo.id;
            set.Anonymous.value = u32::from(on);
            // SAFETY: a correctly sized set packet; screens without HDR refuse it harmlessly.
            if unsafe { DisplayConfigSetDeviceInfo(&set.header) } == 0 {
                done = true;
            }
        }
        if done {
            Ok(())
        } else {
            Err(AppError::Platform("No screen took HDR".into()))
        }
    }

    fn current_mode() -> Option<DEVMODEW> {
        let mut dm = DEVMODEW {
            dmSize: std::mem::size_of::<DEVMODEW>() as u16,
            ..Default::default()
        };
        // SAFETY: fills a local struct for the primary display.
        unsafe { EnumDisplaySettingsW(PCWSTR::null(), ENUM_CURRENT_SETTINGS, &mut dm) }
            .as_bool()
            .then_some(dm)
    }

    fn rates_at(width: u32, height: u32) -> Vec<u32> {
        let mut out = Vec::new();
        for i in 0.. {
            let mut dm = DEVMODEW {
                dmSize: std::mem::size_of::<DEVMODEW>() as u16,
                ..Default::default()
            };
            // SAFETY: enumerates modes into a local struct until Windows says there are no more.
            if !unsafe { EnumDisplaySettingsW(PCWSTR::null(), ENUM_DISPLAY_SETTINGS_MODE(i), &mut dm) }
                .as_bool()
            {
                break;
            }
            if dm.dmPelsWidth == width && dm.dmPelsHeight == height && dm.dmDisplayFrequency > 1 {
                out.push(dm.dmDisplayFrequency);
            }
            if i > 2000 {
                break;
            }
        }
        out.sort_unstable();
        out.dedup();
        out
    }

    pub fn set_rate(hz: u32) -> AppResult<()> {
        let mut dm = current_mode().ok_or_else(|| AppError::Platform("Couldn't read the screen".into()))?;
        if !rates_at(dm.dmPelsWidth, dm.dmPelsHeight).contains(&hz) {
            return Err(AppError::InvalidInput(
                "The screen doesn't offer that rate".into(),
            ));
        }
        dm.dmDisplayFrequency = hz;
        dm.dmFields = DM_DISPLAYFREQUENCY;
        // SAFETY: applies a mode Windows listed for this screen.
        let r =
            unsafe { ChangeDisplaySettingsExW(PCWSTR::null(), Some(&dm), None, CDS_UPDATEREGISTRY, None) };
        if r == DISP_CHANGE_SUCCESSFUL {
            Ok(())
        } else {
            Err(AppError::Platform(
                "Windows didn't change the refresh rate".into(),
            ))
        }
    }

    pub fn read() -> DisplayToggles {
        let mode = current_mode();
        DisplayToggles {
            night_light: night_blob().as_deref().and_then(super::night_light_state),
            hdr: hdr_state(),
            hz: mode.map_or(0, |m| m.dmDisplayFrequency),
            rates: mode
                .map(|m| rates_at(m.dmPelsWidth, m.dmPelsHeight))
                .unwrap_or_default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn blob(on: bool) -> Vec<u8> {
        let mut b = vec![
            0x02, 0x00, 0x00, 0x00, 0x8c, 0x5e, 0x9a, 0x2f, 0x01, 0x00, 0xd5, 0xa7, 0x10, 0x01, 0x00, 0x00,
            0x00, 0x2a,
        ];
        b.push(if on { 0x15 } else { 0x13 });
        b.extend([0xc2, 0x0a, 0x00, 0xca]);
        if on {
            b.extend([0x10, 0x00]);
        }
        b.extend([0x14, 0x0e, 0x00, 0x00]);
        b
    }

    #[test]
    fn flips_night_light_both_ways() {
        let off = blob(false);
        let on = flip_night_light(&off, true).unwrap();
        assert_eq!(night_light_state(&on), Some(true));
        assert_eq!(on.len(), off.len() + 2);
        assert_eq!(&on[23..25], &[0x10, 0x00]);
        assert_eq!(on[10], 0xd6, "counter bumped");
        let back = flip_night_light(&on, false).unwrap();
        assert_eq!(night_light_state(&back), Some(false));
        assert_eq!(back.len(), off.len());
        assert_eq!(flip_night_light(&off, false).unwrap(), off, "already off");
        assert!(flip_night_light(&[1, 2, 3], true).is_none());
    }
}
