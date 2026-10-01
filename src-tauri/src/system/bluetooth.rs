//! Paired Bluetooth devices connecting and disconnecting, through a WinRT device watcher
//! (Windows reports each change; nothing polls). Battery comes from the property Windows
//! exposes for headsets and earbuds that report it.

use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    time::Duration,
};

use windows::{
    core::{Interface, GUID, HSTRING},
    Devices::Enumeration::{
        DeviceInformation, DeviceInformationKind, DeviceInformationUpdate, DeviceWatcher,
    },
    Foundation::{IPropertyValue, TypedEventHandler},
    Win32::System::Com::{CoInitializeEx, COINIT_MULTITHREADED},
};

use windows_collections::{IIterable, IMapView};

use super::BluetoothDevice;

/// Paired classic and Low Energy Bluetooth devices.
const PAIRED_BLUETOOTH: &str = "(System.Devices.Aep.ProtocolId:=\"{e0cbf06c-cd8b-4647-bb8a-263b43f0f974}\" OR System.Devices.Aep.ProtocolId:=\"{bb7bb05e-5972-42b5-94fc-76eaa7084d49}\") AND System.Devices.Aep.IsPaired:=System.StructuredQueryType.Boolean#True";
const IS_CONNECTED: &str = "System.Devices.Aep.IsConnected";
const CONTAINER_ID: &str = "System.Devices.Aep.ContainerId";
/// DEVPKEY_Bluetooth_Battery, on the device's PnP nodes.
const BATTERY: &str = "{104EA319-6EE2-4701-BD47-8DDBF425BBE5} 2";
/// Headsets report their battery a moment after connecting.
const BATTERY_DELAY: Duration = Duration::from_millis(2500);

struct Known {
    name: String,
    connected: bool,
    container: Option<GUID>,
}

#[derive(Clone, Default)]
pub struct Bluetooth {
    devices: Arc<Mutex<HashMap<String, Known>>>,
}

impl Bluetooth {
    /// Connected devices right now, with battery where reported.
    pub fn connected(&self) -> Vec<BluetoothDevice> {
        let list: Vec<(String, Option<GUID>)> = self
            .devices
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .values()
            .filter(|d| d.connected)
            .map(|d| (d.name.clone(), d.container))
            .collect();
        list.into_iter()
            .map(|(name, container)| BluetoothDevice {
                name,
                connected: true,
                battery: container.and_then(battery),
            })
            .collect()
    }
}

fn prop<T>(
    props: &IMapView<HSTRING, windows::core::IInspectable>,
    key: &str,
    read: impl Fn(&IPropertyValue) -> windows::core::Result<T>,
) -> Option<T> {
    let value = props.Lookup(&HSTRING::from(key)).ok()?;
    read(&value.cast::<IPropertyValue>().ok()?).ok()
}

/// Battery percent for a device, from any of its PnP nodes that reports it.
fn battery(container: GUID) -> Option<u8> {
    let filter = HSTRING::from(format!("System.Devices.ContainerId:=\"{{{container:?}}}\""));
    let props = IIterable::<HSTRING>::from(vec![HSTRING::from(BATTERY)]);
    let nodes = DeviceInformation::FindAllAsyncWithKindAqsFilterAndAdditionalProperties(
        &filter,
        &props,
        DeviceInformationKind::Device,
    )
    .ok()?
    .get()
    .ok()?;
    (0..nodes.Size().ok()?)
        .filter_map(|i| nodes.GetAt(i).ok())
        .filter_map(|node| node.Properties().ok())
        .filter_map(|p| prop(&p, BATTERY, |v| v.GetUInt8()))
        .filter(|b| *b <= 100)
        .max()
}

/// Start watching; `report` runs (on a background thread) when a paired device connects or
/// disconnects after startup.
pub fn start(report: impl Fn(BluetoothDevice) + Send + Sync + 'static) -> Bluetooth {
    let bluetooth = Bluetooth::default();
    let devices = bluetooth.devices.clone();
    let report = Arc::new(report);
    std::thread::Builder::new()
        .name("bluetooth".into())
        .spawn(move || {
            if let Err(error) = run(devices, report) {
                tracing::warn!(%error, "bluetooth display unavailable");
            }
        })
        .ok();
    bluetooth
}

fn run(
    devices: Arc<Mutex<HashMap<String, Known>>>,
    report: Arc<dyn Fn(BluetoothDevice) + Send + Sync>,
) -> windows::core::Result<()> {
    // SAFETY: this thread joins the multithreaded apartment for its whole life.
    unsafe { CoInitializeEx(None, COINIT_MULTITHREADED).ok()? };
    let props = IIterable::<HSTRING>::from(vec![HSTRING::from(IS_CONNECTED), HSTRING::from(CONTAINER_ID)]);
    let watcher = DeviceInformation::CreateWatcherWithKindAqsFilterAndAdditionalProperties(
        &HSTRING::from(PAIRED_BLUETOOTH),
        &props,
        DeviceInformationKind::AssociationEndpoint,
    )?;
    // Devices found during the first enumeration were already connected (or not) before
    // Notch started: no card for those.
    let ready = Arc::new(AtomicBool::new(false));

    let added = devices.clone();
    watcher.Added(&TypedEventHandler::<DeviceWatcher, DeviceInformation>::new(
        move |_, info| {
            if let Some(info) = info.as_ref() {
                let props = info.Properties()?;
                added.lock().unwrap_or_else(|e| e.into_inner()).insert(
                    info.Id()?.to_string(),
                    Known {
                        name: info.Name()?.to_string().chars().take(48).collect(),
                        connected: prop(&props, IS_CONNECTED, |v| v.GetBoolean()).unwrap_or(false),
                        container: prop(&props, CONTAINER_ID, |v| v.GetGuid()),
                    },
                );
            }
            Ok(())
        },
    ))?;

    let updated = devices.clone();
    let updated_ready = ready.clone();
    watcher.Updated(&TypedEventHandler::<DeviceWatcher, DeviceInformationUpdate>::new(
        move |_, update| {
            let Some(update) = update.as_ref() else {
                return Ok(());
            };
            let Some(connected) = prop(&update.Properties()?, IS_CONNECTED, |v| v.GetBoolean()) else {
                return Ok(());
            };
            let changed = {
                let mut devices = updated.lock().unwrap_or_else(|e| e.into_inner());
                let Some(known) = devices.get_mut(&update.Id()?.to_string()) else {
                    return Ok(());
                };
                let changed = known.connected != connected;
                known.connected = connected;
                changed.then(|| (known.name.clone(), known.container))
            };
            if let (Some((name, container)), true) = (changed, updated_ready.load(Ordering::SeqCst)) {
                let report = report.clone();
                // Off the watcher's thread: the battery lookup waits for the headset to report.
                std::thread::spawn(move || {
                    let battery = if connected {
                        std::thread::sleep(BATTERY_DELAY);
                        container.and_then(battery)
                    } else {
                        None
                    };
                    report(BluetoothDevice {
                        name,
                        connected,
                        battery,
                    });
                });
            }
            Ok(())
        },
    ))?;

    let removed = devices.clone();
    watcher.Removed(&TypedEventHandler::<DeviceWatcher, DeviceInformationUpdate>::new(
        move |_, update| {
            if let Some(update) = update.as_ref() {
                removed
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .remove(&update.Id()?.to_string());
            }
            Ok(())
        },
    ))?;

    let completed = ready.clone();
    watcher.EnumerationCompleted(
        &TypedEventHandler::<DeviceWatcher, windows::core::IInspectable>::new(move |_, _| {
            completed.store(true, Ordering::SeqCst);
            Ok(())
        }),
    )?;

    watcher.Start()?;
    // Keep the watcher (and this apartment) alive for the life of the app.
    loop {
        std::thread::park();
    }
}
