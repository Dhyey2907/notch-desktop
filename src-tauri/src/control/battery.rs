//! Battery details straight from the battery driver (`IOCTL_BATTERY_QUERY_*`), which needs
//! no admin rights: charge, charge rate, design and full-charge capacity, cycle count.

use windows::{
    core::PCWSTR,
    Win32::{
        Devices::DeviceAndDriverInstallation::{
            SetupDiDestroyDeviceInfoList, SetupDiEnumDeviceInterfaces, SetupDiGetClassDevsW,
            SetupDiGetDeviceInterfaceDetailW, DIGCF_DEVICEINTERFACE, DIGCF_PRESENT, SP_DEVICE_INTERFACE_DATA,
            SP_DEVICE_INTERFACE_DETAIL_DATA_W,
        },
        Foundation::{CloseHandle, HANDLE},
        Storage::FileSystem::{
            CreateFileW, FILE_ATTRIBUTE_NORMAL, FILE_GENERIC_READ, FILE_GENERIC_WRITE, FILE_SHARE_READ,
            FILE_SHARE_WRITE, OPEN_EXISTING,
        },
        System::{
            Power::{
                BatteryInformation, BATTERY_INFORMATION, BATTERY_POWER_ON_LINE, BATTERY_QUERY_INFORMATION,
                BATTERY_STATUS, BATTERY_WAIT_STATUS, GUID_DEVICE_BATTERY, IOCTL_BATTERY_QUERY_INFORMATION,
                IOCTL_BATTERY_QUERY_STATUS, IOCTL_BATTERY_QUERY_TAG,
            },
            IO::DeviceIoControl,
        },
    },
};

use super::{health, minutes_left, PowerInfo};

/// `BATTERY_CHARGING` in `BATTERY_STATUS.PowerState`.
const CHARGING: u32 = 0x4;
/// Capacity and rate are "unknown" when they read as this.
const UNKNOWN: u32 = 0xFFFF_FFFF;

struct Handle(HANDLE);

impl Drop for Handle {
    fn drop(&mut self) {
        // SAFETY: closing a handle this module opened.
        unsafe {
            let _ = CloseHandle(self.0);
        }
    }
}

/// Path of the first battery device, if the PC has one.
fn battery_path() -> Option<Vec<u16>> {
    // SAFETY: SetupAPI enumeration with locally owned buffers; the list is destroyed below.
    unsafe {
        let set = SetupDiGetClassDevsW(
            Some(&GUID_DEVICE_BATTERY),
            PCWSTR::null(),
            None,
            DIGCF_PRESENT | DIGCF_DEVICEINTERFACE,
        )
        .ok()?;
        let mut iface = SP_DEVICE_INTERFACE_DATA {
            cbSize: std::mem::size_of::<SP_DEVICE_INTERFACE_DATA>() as u32,
            ..Default::default()
        };
        let path = (|| {
            SetupDiEnumDeviceInterfaces(set, None, &GUID_DEVICE_BATTERY, 0, &mut iface).ok()?;
            let mut needed = 0;
            let _ = SetupDiGetDeviceInterfaceDetailW(set, &iface, None, 0, Some(&mut needed), None);
            if needed == 0 {
                return None;
            }
            // u64 storage keeps the struct aligned.
            let mut buffer = vec![0u64; (needed as usize).div_ceil(8)];
            let detail = buffer.as_mut_ptr().cast::<SP_DEVICE_INTERFACE_DETAIL_DATA_W>();
            (*detail).cbSize = std::mem::size_of::<SP_DEVICE_INTERFACE_DETAIL_DATA_W>() as u32;
            SetupDiGetDeviceInterfaceDetailW(set, &iface, Some(detail), needed, None, None).ok()?;
            let start = std::ptr::addr_of!((*detail).DevicePath).cast::<u16>();
            let max = (needed as usize - 4) / 2;
            let len = (0..max).find(|&i| *start.add(i) == 0).unwrap_or(max);
            let mut path = std::slice::from_raw_parts(start, len).to_vec();
            path.push(0);
            Some(path)
        })();
        let _ = SetupDiDestroyDeviceInfoList(set);
        path
    }
}

fn ioctl<I, O: Default>(device: &Handle, code: u32, input: &I) -> Option<O> {
    let mut out = O::default();
    let mut returned = 0;
    // SAFETY: input and output point to locals of the sizes passed.
    unsafe {
        DeviceIoControl(
            device.0,
            code,
            Some(std::ptr::from_ref(input).cast()),
            std::mem::size_of::<I>() as u32,
            Some(std::ptr::from_mut(&mut out).cast()),
            std::mem::size_of::<O>() as u32,
            Some(&mut returned),
            None,
        )
        .ok()?;
    }
    Some(out)
}

pub fn read() -> Option<PowerInfo> {
    let path = battery_path()?;
    // SAFETY: opening the battery device by the path Windows gave us.
    let device = unsafe {
        CreateFileW(
            PCWSTR(path.as_ptr()),
            (FILE_GENERIC_READ | FILE_GENERIC_WRITE).0,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            None,
            OPEN_EXISTING,
            FILE_ATTRIBUTE_NORMAL,
            None,
        )
    }
    .ok()
    .map(Handle)?;

    let tag: u32 = ioctl(&device, IOCTL_BATTERY_QUERY_TAG, &0u32)?;
    if tag == 0 {
        return None;
    }
    let info: BATTERY_INFORMATION = ioctl(
        &device,
        IOCTL_BATTERY_QUERY_INFORMATION,
        &BATTERY_QUERY_INFORMATION {
            BatteryTag: tag,
            InformationLevel: BatteryInformation,
            AtRate: 0,
        },
    )?;
    let status: BATTERY_STATUS = ioctl(
        &device,
        IOCTL_BATTERY_QUERY_STATUS,
        &BATTERY_WAIT_STATUS {
            BatteryTag: tag,
            ..Default::default()
        },
    )?;

    let full = info.FullChargedCapacity;
    let capacity = status.Capacity;
    if full == 0 || full == UNKNOWN || capacity == UNKNOWN {
        return None;
    }
    let percent = ((capacity as f64 / full as f64) * 100.0).round().min(100.0) as u8;
    let plugged = status.PowerState & BATTERY_POWER_ON_LINE != 0;
    let charging = status.PowerState & CHARGING != 0;
    // `Rate` is mW: positive while charging, negative while discharging.
    let rate = status.Rate;
    let minutes = if rate == i32::MIN {
        None // unknown
    } else if charging || !plugged {
        minutes_left(capacity, full, rate)
    } else {
        None
    };
    Some(PowerInfo {
        percent,
        plugged,
        charging,
        minutes,
        health: health(full, info.DesignedCapacity),
        cycles: (info.CycleCount > 0).then_some(info.CycleCount),
    })
}
