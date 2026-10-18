//! The sound mixer: each app's own volume (Core Audio sessions on the default output) and
//! switching which device sound comes out of. Switching uses `IPolicyConfig`, the interface the
//! Windows sound panel itself uses (not documented, stable since Windows 7).

use serde::Serialize;
use ts_rs::TS;

use crate::error::{AppError, AppResult};

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct AudioDevice {
    /// Windows' id for the device.
    pub id: String,
    pub name: String,
    /// Sound goes here now.
    pub default: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct AppVolume {
    pub pid: u32,
    /// "Spotify", "Chrome"…
    pub name: String,
    /// 0..1
    pub level: f32,
    pub muted: bool,
    /// Making sound right now.
    pub active: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Mixer {
    pub devices: Vec<AudioDevice>,
    /// Apps with sound on the default device, loudest first.
    pub apps: Vec<AppVolume>,
}

/// Merge sessions of the same app (Chrome has several) and drop system sounds.
pub fn merge(apps: Vec<AppVolume>) -> Vec<AppVolume> {
    let mut out: Vec<AppVolume> = Vec::new();
    for app in apps {
        match out.iter_mut().find(|a| a.name.eq_ignore_ascii_case(&app.name)) {
            Some(a) => {
                a.active |= app.active;
                a.level = a.level.max(app.level);
                a.muted &= app.muted;
            }
            None => out.push(app),
        }
    }
    out.sort_by(|a, b| {
        b.active
            .cmp(&a.active)
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });
    out
}

// The COM interface below keeps Windows' method names.
#[cfg(windows)]
#[allow(non_snake_case)]
mod win {
    use windows::{
        core::{interface, IUnknown, IUnknown_Vtbl, Interface, HRESULT, PCWSTR},
        Win32::{
            Devices::FunctionDiscovery::PKEY_Device_FriendlyName,
            Media::Audio::{
                eConsole, eMultimedia, eRender, AudioSessionStateActive, ERole, IAudioSessionControl2,
                IAudioSessionManager2, IMMDevice, IMMDeviceEnumerator, ISimpleAudioVolume,
                MMDeviceEnumerator, DEVICE_STATE_ACTIVE,
            },
            System::Com::{
                CoCreateInstance, CoInitializeEx, CoTaskMemFree, CLSCTX_ALL, COINIT_MULTITHREADED, STGM_READ,
            },
        },
    };

    use super::{AppVolume, AudioDevice, Mixer};
    use crate::error::{AppError, AppResult};

    /// Windows' own default-device switcher (CLSID `PolicyConfigClient`).
    const POLICY_CONFIG_CLIENT: windows::core::GUID =
        windows::core::GUID::from_u128(0x870af99c_171d_4f9e_af0d_e63df40c2bc9);

    /// Only the slot order matters: `SetDefaultEndpoint` is the eleventh method. The names
    /// follow the COM interface.
    #[interface("f8679f50-850a-41cf-9c72-430f290290c8")]
    unsafe trait IPolicyConfig: IUnknown {
        fn GetMixFormat(&self, a: PCWSTR, b: *mut *mut core::ffi::c_void) -> HRESULT;
        fn GetDeviceFormat(&self, a: PCWSTR, b: i32, c: *mut *mut core::ffi::c_void) -> HRESULT;
        fn ResetDeviceFormat(&self, a: PCWSTR) -> HRESULT;
        fn SetDeviceFormat(&self, a: PCWSTR, b: *mut core::ffi::c_void, c: *mut core::ffi::c_void)
            -> HRESULT;
        fn GetProcessingPeriod(&self, a: PCWSTR, b: i32, c: *mut i64, d: *mut i64) -> HRESULT;
        fn SetProcessingPeriod(&self, a: PCWSTR, b: *mut i64) -> HRESULT;
        fn GetShareMode(&self, a: PCWSTR, b: *mut core::ffi::c_void) -> HRESULT;
        fn SetShareMode(&self, a: PCWSTR, b: *mut core::ffi::c_void) -> HRESULT;
        fn GetPropertyValue(
            &self,
            a: PCWSTR,
            b: *const core::ffi::c_void,
            c: *mut core::ffi::c_void,
        ) -> HRESULT;
        fn SetPropertyValue(
            &self,
            a: PCWSTR,
            b: *const core::ffi::c_void,
            c: *const core::ffi::c_void,
        ) -> HRESULT;
        fn SetDefaultEndpoint(&self, id: PCWSTR, role: ERole) -> HRESULT;
        fn SetEndpointVisibility(&self, id: PCWSTR, visible: i32) -> HRESULT;
    }

    fn platform(e: windows::core::Error) -> AppError {
        AppError::Platform(e.to_string())
    }

    fn enumerator() -> AppResult<IMMDeviceEnumerator> {
        // SAFETY: COM setup on this (blocking-pool) thread, then a plain object creation.
        unsafe {
            let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
            CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL).map_err(platform)
        }
    }

    fn id_of(device: &IMMDevice) -> Option<String> {
        // SAFETY: the id string is COM-allocated; read it, then free it once.
        unsafe {
            let raw = device.GetId().ok()?;
            let id = raw.to_string().ok();
            CoTaskMemFree(Some(raw.0 as *const _));
            id
        }
    }

    fn name_of(device: &IMMDevice) -> String {
        // SAFETY: reads one property from the device's store.
        unsafe {
            device
                .OpenPropertyStore(STGM_READ)
                .and_then(|store| store.GetValue(&PKEY_Device_FriendlyName))
                .map(|value| value.to_string())
                .map(|name| crate::system::audio::short_device_name(&name))
                .unwrap_or_default()
        }
    }

    pub fn read() -> AppResult<Mixer> {
        let e = enumerator()?;
        // SAFETY: COM calls on live, reference-counted interfaces.
        unsafe {
            let default = e
                .GetDefaultAudioEndpoint(eRender, eMultimedia)
                .map_err(platform)?;
            let default_id = id_of(&default);
            let list = e
                .EnumAudioEndpoints(eRender, DEVICE_STATE_ACTIVE)
                .map_err(platform)?;
            let mut devices = Vec::new();
            for i in 0..list.GetCount().map_err(platform)? {
                let Ok(device) = list.Item(i) else { continue };
                let Some(id) = id_of(&device) else { continue };
                devices.push(AudioDevice {
                    default: Some(&id) == default_id.as_ref(),
                    name: name_of(&device),
                    id,
                });
            }
            let manager: IAudioSessionManager2 = default.Activate(CLSCTX_ALL, None).map_err(platform)?;
            let sessions = manager.GetSessionEnumerator().map_err(platform)?;
            let mut apps = Vec::new();
            for i in 0..sessions.GetCount().map_err(platform)? {
                let Ok(control) = sessions.GetSession(i) else {
                    continue;
                };
                let Ok(control) = control.cast::<IAudioSessionControl2>() else {
                    continue;
                };
                // System sounds (the "ding") have their own slider in Windows; leave them out.
                if control.IsSystemSoundsSession() == windows::Win32::Foundation::S_OK {
                    continue;
                }
                let Ok(pid) = control.GetProcessId() else { continue };
                if pid == 0 {
                    continue;
                }
                let Ok(volume) = control.cast::<ISimpleAudioVolume>() else {
                    continue;
                };
                let Some(exe) = crate::clipboard::win::exe_of(pid) else {
                    continue;
                };
                apps.push(AppVolume {
                    pid,
                    name: crate::clipboard::app_label(&exe),
                    level: volume.GetMasterVolume().unwrap_or(1.0),
                    muted: volume.GetMute().map(|m| m.as_bool()).unwrap_or(false),
                    active: control.GetState().is_ok_and(|s| s == AudioSessionStateActive),
                });
            }
            Ok(Mixer {
                devices,
                apps: super::merge(apps),
            })
        }
    }

    /// Every session of this app on the default device (Chrome has several).
    fn volumes_of(pid: u32) -> AppResult<Vec<ISimpleAudioVolume>> {
        let e = enumerator()?;
        // SAFETY: COM calls on live, reference-counted interfaces.
        unsafe {
            let default = e
                .GetDefaultAudioEndpoint(eRender, eMultimedia)
                .map_err(platform)?;
            let manager: IAudioSessionManager2 = default.Activate(CLSCTX_ALL, None).map_err(platform)?;
            let sessions = manager.GetSessionEnumerator().map_err(platform)?;
            let name = crate::clipboard::win::exe_of(pid).map(|e| crate::clipboard::app_label(&e));
            let mut out = Vec::new();
            for i in 0..sessions.GetCount().map_err(platform)? {
                let Ok(control) = sessions.GetSession(i) else {
                    continue;
                };
                let Ok(control) = control.cast::<IAudioSessionControl2>() else {
                    continue;
                };
                let Ok(other) = control.GetProcessId() else {
                    continue;
                };
                let same = other == pid
                    || (name.is_some()
                        && crate::clipboard::win::exe_of(other).map(|e| crate::clipboard::app_label(&e))
                            == name);
                if same {
                    if let Ok(v) = control.cast::<ISimpleAudioVolume>() {
                        out.push(v);
                    }
                }
            }
            if out.is_empty() {
                return Err(AppError::InvalidInput("That app isn't playing sound".into()));
            }
            Ok(out)
        }
    }

    pub fn set_volume(pid: u32, level: f32) -> AppResult<()> {
        for v in volumes_of(pid)? {
            // SAFETY: a plain COM call on a live interface.
            unsafe { v.SetMasterVolume(level.clamp(0.0, 1.0), std::ptr::null()) }.map_err(platform)?;
        }
        Ok(())
    }

    pub fn set_mute(pid: u32, muted: bool) -> AppResult<()> {
        for v in volumes_of(pid)? {
            // SAFETY: a plain COM call on a live interface.
            unsafe { v.SetMute(muted, std::ptr::null()) }.map_err(platform)?;
        }
        Ok(())
    }

    pub fn set_output(id: &str) -> AppResult<()> {
        let _ = enumerator()?;
        let wide = windows::core::HSTRING::from(id);
        // SAFETY: creates Windows' own policy object and calls it with a valid device id.
        unsafe {
            let policy: IPolicyConfig =
                CoCreateInstance(&POLICY_CONFIG_CLIENT, None, CLSCTX_ALL).map_err(platform)?;
            for role in [
                eConsole,
                eMultimedia,
                windows::Win32::Media::Audio::eCommunications,
            ] {
                policy
                    .SetDefaultEndpoint(PCWSTR(wide.as_ptr()), role)
                    .ok()
                    .map_err(platform)?;
            }
        }
        Ok(())
    }
}

pub fn read() -> AppResult<Mixer> {
    #[cfg(windows)]
    return win::read();
    #[cfg(not(windows))]
    Err(AppError::Platform("not supported".into()))
}

pub fn set_app_volume(pid: u32, level: f32) -> AppResult<()> {
    if !level.is_finite() {
        return Err(AppError::InvalidInput("invalid volume".into()));
    }
    #[cfg(windows)]
    return win::set_volume(pid, level);
    #[cfg(not(windows))]
    {
        let _ = pid;
        Err(AppError::Platform("not supported".into()))
    }
}

pub fn set_app_mute(pid: u32, muted: bool) -> AppResult<()> {
    #[cfg(windows)]
    return win::set_mute(pid, muted);
    #[cfg(not(windows))]
    {
        let _ = (pid, muted);
        Err(AppError::Platform("not supported".into()))
    }
}

/// Send sound to this device (Windows' id for it).
pub fn set_output(id: &str) -> AppResult<()> {
    if id.is_empty() || id.len() > 300 {
        return Err(AppError::InvalidInput("invalid device".into()));
    }
    #[cfg(windows)]
    return win::set_output(id);
    #[cfg(not(windows))]
    Err(AppError::Platform("not supported".into()))
}

#[cfg(test)]
mod tests {
    use super::{merge, AppVolume};

    fn app(name: &str, level: f32, active: bool) -> AppVolume {
        AppVolume {
            pid: 1,
            name: name.into(),
            level,
            muted: false,
            active,
        }
    }

    #[test]
    fn merges_an_apps_sessions_and_puts_playing_ones_first() {
        let out = merge(vec![
            app("chrome", 0.4, false),
            app("Spotify", 1.0, false),
            app("Chrome", 0.8, true),
        ]);
        assert_eq!(out.len(), 2);
        assert_eq!(out[0].name, "chrome");
        assert!(out[0].active);
        assert!((out[0].level - 0.8).abs() < f32::EPSILON);
    }
}
