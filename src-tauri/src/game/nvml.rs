//! GPU temperature and load from NVIDIA's NVML (`nvml.dll`, installed with the driver).
//! Loaded at run time, so PCs without an NVIDIA GPU simply show no GPU numbers.

use std::sync::OnceLock;

use windows::{
    core::{s, w},
    Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryW},
};

type Init = unsafe extern "C" fn() -> i32;
type HandleByIndex = unsafe extern "C" fn(u32, *mut *mut core::ffi::c_void) -> i32;
type Temperature = unsafe extern "C" fn(*mut core::ffi::c_void, i32, *mut u32) -> i32;
type Utilization = unsafe extern "C" fn(*mut core::ffi::c_void, *mut [u32; 2]) -> i32;

struct Nvml {
    device: usize,
    temperature: Temperature,
    utilization: Utilization,
}

const NVML_SUCCESS: i32 = 0;
const NVML_TEMPERATURE_GPU: i32 = 0;

static NVML: OnceLock<Option<Nvml>> = OnceLock::new();

fn load() -> Option<Nvml> {
    // SAFETY: loading the driver's NVML and resolving documented entry points with matching
    // C signatures; the library stays loaded for the life of the app.
    unsafe {
        let lib = LoadLibraryW(w!("nvml.dll")).ok()?;
        let init: Init = std::mem::transmute(GetProcAddress(lib, s!("nvmlInit_v2"))?);
        let by_index: HandleByIndex =
            std::mem::transmute(GetProcAddress(lib, s!("nvmlDeviceGetHandleByIndex_v2"))?);
        let temperature: Temperature =
            std::mem::transmute(GetProcAddress(lib, s!("nvmlDeviceGetTemperature"))?);
        let utilization: Utilization =
            std::mem::transmute(GetProcAddress(lib, s!("nvmlDeviceGetUtilizationRates"))?);
        if init() != NVML_SUCCESS {
            return None;
        }
        let mut device = std::ptr::null_mut();
        if by_index(0, &mut device) != NVML_SUCCESS || device.is_null() {
            return None;
        }
        Some(Nvml {
            device: device as usize,
            temperature,
            utilization,
        })
    }
}

/// (temperature °C, load %) of the first NVIDIA GPU, when there is one.
pub fn gpu() -> (Option<u32>, Option<u32>) {
    let Some(nvml) = NVML.get_or_init(load) else {
        return (None, None);
    };
    let device = nvml.device as *mut core::ffi::c_void;
    let mut temp = 0u32;
    let mut util = [0u32; 2];
    // SAFETY: calls into the loaded NVML with a valid device handle and out-pointers.
    unsafe {
        let t = ((nvml.temperature)(device, NVML_TEMPERATURE_GPU, &mut temp) == NVML_SUCCESS).then_some(temp);
        let u = ((nvml.utilization)(device, &mut util) == NVML_SUCCESS).then_some(util[0].min(100));
        (t, u)
    }
}
