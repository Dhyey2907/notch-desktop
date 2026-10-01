//! Speaker volume through Core Audio: Windows calls us on every volume or mute change and when
//! the default output device switches (headphones plugged in, say). Nothing polls. One thread
//! owns the COM objects; changes from the notch (dragging the bar) are sent to it.

use std::sync::mpsc::{channel, Receiver, Sender};

use windows::{
    core::{implement, GUID, PCWSTR},
    Win32::{
        Devices::FunctionDiscovery::PKEY_Device_FriendlyName,
        Foundation::PROPERTYKEY,
        Media::Audio::{
            eMultimedia, eRender, EDataFlow, ERole,
            Endpoints::{
                IAudioEndpointVolume, IAudioEndpointVolumeCallback, IAudioEndpointVolumeCallback_Impl,
            },
            IMMDeviceEnumerator, IMMNotificationClient, IMMNotificationClient_Impl, MMDeviceEnumerator,
            AUDIO_VOLUME_NOTIFICATION_DATA, DEVICE_STATE,
        },
        System::Com::{CoCreateInstance, CoInitializeEx, CLSCTX_ALL, COINIT_MULTITHREADED, STGM_READ},
    },
};

use super::SoundState;

/// Marks volume changes made from the notch.
const NOTCH_CONTEXT: GUID = GUID::from_u128(0x6e6f7463_6800_4e6f_7463_68566f6c756d);

/// Why the state was reported.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cause {
    /// Read at startup: nothing changed.
    Initial,
    /// Volume or mute changed (keys, the Windows flyout, an app, or the notch).
    Changed,
    /// Sound now goes to a different device.
    DeviceSwitched,
}

enum Msg {
    Notify { level: f32, muted: bool },
    DefaultChanged,
    SetLevel(f32),
    ToggleMute,
}

/// Handle for changing volume from the notch.
#[derive(Clone)]
pub struct Audio {
    tx: Sender<Msg>,
}

impl Audio {
    pub fn set_level(&self, level: f32) {
        let _ = self.tx.send(Msg::SetLevel(level.clamp(0.0, 1.0)));
    }

    pub fn toggle_mute(&self) {
        let _ = self.tx.send(Msg::ToggleMute);
    }
}

#[implement(IAudioEndpointVolumeCallback)]
struct VolumeCallback {
    tx: Sender<Msg>,
}

impl IAudioEndpointVolumeCallback_Impl for VolumeCallback_Impl {
    fn OnNotify(&self, data: *mut AUDIO_VOLUME_NOTIFICATION_DATA) -> windows::core::Result<()> {
        // SAFETY: Windows passes a valid notification for the duration of the call.
        if let Some(data) = unsafe { data.as_ref() } {
            let _ = self.tx.send(Msg::Notify {
                level: data.fMasterVolume,
                muted: data.bMuted.as_bool(),
            });
        }
        Ok(())
    }
}

#[implement(IMMNotificationClient)]
struct DeviceCallback {
    tx: Sender<Msg>,
}

impl IMMNotificationClient_Impl for DeviceCallback_Impl {
    fn OnDeviceStateChanged(&self, _: &PCWSTR, _: DEVICE_STATE) -> windows::core::Result<()> {
        Ok(())
    }
    fn OnDeviceAdded(&self, _: &PCWSTR) -> windows::core::Result<()> {
        Ok(())
    }
    fn OnDeviceRemoved(&self, _: &PCWSTR) -> windows::core::Result<()> {
        Ok(())
    }
    fn OnDefaultDeviceChanged(&self, flow: EDataFlow, role: ERole, _: &PCWSTR) -> windows::core::Result<()> {
        if flow == eRender && role == eMultimedia {
            let _ = self.tx.send(Msg::DefaultChanged);
        }
        Ok(())
    }
    fn OnPropertyValueChanged(&self, _: &PCWSTR, _: &PROPERTYKEY) -> windows::core::Result<()> {
        Ok(())
    }
}

/// Start watching; `report` runs on the audio thread for every change.
pub fn start(report: impl Fn(SoundState, Cause) + Send + 'static) -> Audio {
    let (tx, rx) = channel();
    let audio = Audio { tx: tx.clone() };
    std::thread::Builder::new()
        .name("audio".into())
        .spawn(move || {
            if let Err(error) = run(tx, rx, report) {
                tracing::warn!(%error, "volume display unavailable");
            }
        })
        .ok();
    audio
}

struct Endpoint {
    volume: IAudioEndpointVolume,
    callback: IAudioEndpointVolumeCallback,
    device: Option<String>,
}

impl Endpoint {
    fn state(&self) -> SoundState {
        // SAFETY: plain getters on a live endpoint.
        unsafe {
            SoundState {
                level: self.volume.GetMasterVolumeLevelScalar().unwrap_or(0.0),
                muted: self.volume.GetMute().map(|m| m.as_bool()).unwrap_or(false),
                device: self.device.clone(),
            }
        }
    }
}

impl Drop for Endpoint {
    fn drop(&mut self) {
        // SAFETY: unregistering the callback this endpoint registered.
        unsafe {
            let _ = self.volume.UnregisterControlChangeNotify(&self.callback);
        }
    }
}

fn attach(enumerator: &IMMDeviceEnumerator, tx: &Sender<Msg>) -> windows::core::Result<Endpoint> {
    // SAFETY: COM calls on this MTA thread; every interface is reference-counted.
    unsafe {
        let device = enumerator.GetDefaultAudioEndpoint(eRender, eMultimedia)?;
        let volume: IAudioEndpointVolume = device.Activate(CLSCTX_ALL, None)?;
        let callback: IAudioEndpointVolumeCallback = VolumeCallback { tx: tx.clone() }.into();
        volume.RegisterControlChangeNotify(&callback)?;
        let name = device
            .OpenPropertyStore(STGM_READ)
            .and_then(|store| store.GetValue(&PKEY_Device_FriendlyName))
            .ok()
            .map(|value| value.to_string())
            .map(|name| short_device_name(&name))
            .filter(|name| !name.is_empty());
        Ok(Endpoint {
            volume,
            callback,
            device: name,
        })
    }
}

/// "Speakers (Realtek(R) Audio)" -> "Speakers"; "Headphones (Galaxy Buds2 Pro)" -> "Galaxy Buds2 Pro".
pub fn short_device_name(name: &str) -> String {
    let name = name.trim();
    match name.split_once(" (") {
        Some((kind, rest)) => {
            let inner = rest.strip_suffix(')').unwrap_or(rest).trim();
            let generic = [
                "realtek",
                "high definition audio",
                "intel",
                "amd",
                "nvidia",
                "conexant",
            ]
            .iter()
            .any(|v| inner.to_lowercase().contains(v));
            if generic || inner.is_empty() {
                kind.to_owned()
            } else {
                inner.to_owned()
            }
        }
        None => name.to_owned(),
    }
    .chars()
    .take(40)
    .collect()
}

fn run(tx: Sender<Msg>, rx: Receiver<Msg>, report: impl Fn(SoundState, Cause)) -> windows::core::Result<()> {
    // SAFETY: this thread joins the multithreaded apartment for its whole life.
    unsafe { CoInitializeEx(None, COINIT_MULTITHREADED).ok()? };
    // SAFETY: creating the system device enumerator.
    let enumerator: IMMDeviceEnumerator = unsafe { CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)? };
    let devices: IMMNotificationClient = DeviceCallback { tx: tx.clone() }.into();
    // SAFETY: registered for the life of the thread (the app).
    unsafe { enumerator.RegisterEndpointNotificationCallback(&devices)? };

    let mut endpoint = attach(&enumerator, &tx).ok();
    if let Some(e) = &endpoint {
        report(e.state(), Cause::Initial);
    }
    for msg in rx {
        match msg {
            Msg::Notify { level, muted } => {
                let device = endpoint.as_ref().and_then(|e| e.device.clone());
                report(SoundState { level, muted, device }, Cause::Changed);
            }
            Msg::DefaultChanged => {
                drop(endpoint.take()); // unregister from the old device first
                endpoint = attach(&enumerator, &tx).ok();
                if let Some(e) = &endpoint {
                    report(e.state(), Cause::DeviceSwitched);
                }
            }
            Msg::SetLevel(level) => {
                if let Some(e) = &endpoint {
                    // SAFETY: setting volume on the live endpoint, tagged as ours.
                    unsafe {
                        if e.volume.GetMute().is_ok_and(|m| m.as_bool()) {
                            let _ = e.volume.SetMute(false, &NOTCH_CONTEXT);
                        }
                        let _ = e.volume.SetMasterVolumeLevelScalar(level, &NOTCH_CONTEXT);
                    }
                }
            }
            Msg::ToggleMute => {
                if let Some(e) = &endpoint {
                    // SAFETY: as above.
                    unsafe {
                        let muted = e.volume.GetMute().is_ok_and(|m| m.as_bool());
                        let _ = e.volume.SetMute(!muted, &NOTCH_CONTEXT);
                    }
                }
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::short_device_name;

    #[test]
    fn shortens_device_names() {
        assert_eq!(short_device_name("Speakers (Realtek(R) Audio)"), "Speakers");
        assert_eq!(
            short_device_name("Headphones (Galaxy Buds2 Pro)"),
            "Galaxy Buds2 Pro"
        );
        assert_eq!(
            short_device_name("Speakers (2- High Definition Audio Device)"),
            "Speakers"
        );
        assert_eq!(short_device_name("LG HDR 4K"), "LG HDR 4K");
    }
}
