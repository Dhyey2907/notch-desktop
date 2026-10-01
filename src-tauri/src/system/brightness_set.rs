//! Setting the laptop screen's brightness through WMI (`WmiMonitorBrightnessMethods.
//! WmiSetBrightness`), on one worker thread that keeps the WMI connection. Dragging sends many
//! levels; only the latest waiting one is applied.

use std::sync::{
    mpsc::{channel, Sender},
    Mutex, OnceLock,
};

use windows::{
    core::{BSTR, PCWSTR},
    Win32::System::{
        Com::{
            CoCreateInstance, CoInitializeEx, CoSetProxyBlanket, CLSCTX_INPROC_SERVER, COINIT_MULTITHREADED,
            EOAC_NONE, RPC_C_AUTHN_LEVEL_CALL, RPC_C_IMP_LEVEL_IMPERSONATE,
        },
        Rpc::{RPC_C_AUTHN_WINNT, RPC_C_AUTHZ_NONE},
        Variant::VARIANT,
        Wmi::{
            IWbemClassObject, IWbemLocator, IWbemServices, WbemLocator, WBEM_FLAG_FORWARD_ONLY,
            WBEM_FLAG_RETURN_IMMEDIATELY, WBEM_GENERIC_FLAG_TYPE, WBEM_INFINITE,
        },
    },
};

static WORKER: OnceLock<Mutex<Sender<u8>>> = OnceLock::new();

/// Ask for a new brightness (percent). Returns false if this PC can't set it.
pub fn set(level: u8) -> bool {
    let worker = WORKER.get_or_init(|| {
        let (tx, rx) = channel::<u8>();
        std::thread::Builder::new()
            .name("brightness-set".into())
            .spawn(move || {
                let connection = connect();
                if let Err(error) = &connection {
                    tracing::warn!(%error, "can't set brightness on this PC");
                }
                while let Ok(mut level) = rx.recv() {
                    // Skip to the newest request.
                    while let Ok(newer) = rx.try_recv() {
                        level = newer;
                    }
                    if let Ok(wmi) = &connection {
                        if let Err(error) = wmi.apply(level) {
                            tracing::debug!(%error, "brightness change failed");
                        }
                    }
                }
            })
            .expect("spawn brightness worker");
        Mutex::new(tx)
    });
    worker
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .send(level.min(100))
        .is_ok()
}

struct Wmi {
    services: IWbemServices,
    /// Object path of the built-in display's brightness methods.
    path: BSTR,
    /// Input parameters of `WmiSetBrightness`.
    params: IWbemClassObject,
}

fn connect() -> windows::core::Result<Wmi> {
    // SAFETY: COM calls on this worker's own MTA; interfaces are reference-counted.
    unsafe {
        CoInitializeEx(None, COINIT_MULTITHREADED).ok()?;
        let locator: IWbemLocator = CoCreateInstance(&WbemLocator, None, CLSCTX_INPROC_SERVER)?;
        let services = locator.ConnectServer(
            &BSTR::from("ROOT\\WMI"),
            &BSTR::new(),
            &BSTR::new(),
            &BSTR::new(),
            0,
            &BSTR::new(),
            None,
        )?;
        CoSetProxyBlanket(
            &services,
            RPC_C_AUTHN_WINNT,
            RPC_C_AUTHZ_NONE,
            PCWSTR::null(),
            RPC_C_AUTHN_LEVEL_CALL,
            RPC_C_IMP_LEVEL_IMPERSONATE,
            None,
            EOAC_NONE,
        )?;

        let instances = services.CreateInstanceEnum(
            &BSTR::from("WmiMonitorBrightnessMethods"),
            WBEM_GENERIC_FLAG_TYPE(WBEM_FLAG_FORWARD_ONLY.0 | WBEM_FLAG_RETURN_IMMEDIATELY.0),
            None,
        )?;
        let mut found = [None];
        let mut returned = 0;
        instances.Next(WBEM_INFINITE, &mut found, &mut returned).ok()?;
        let instance = found[0]
            .take()
            .ok_or_else(|| windows::core::Error::from_hresult(windows::Win32::Foundation::E_NOTIMPL))?;
        let mut path = VARIANT::default();
        instance.Get(windows::core::w!("__PATH"), 0, &mut path, None, None)?;
        let path = BSTR::try_from(&path)?;

        let mut class = None;
        services.GetObject(
            &BSTR::from("WmiMonitorBrightnessMethods"),
            WBEM_GENERIC_FLAG_TYPE(0),
            None,
            Some(&mut class),
            None,
        )?;
        let class =
            class.ok_or_else(|| windows::core::Error::from_hresult(windows::Win32::Foundation::E_FAIL))?;
        let mut signature = None;
        class.GetMethod(
            windows::core::w!("WmiSetBrightness"),
            0,
            &mut signature,
            std::ptr::null_mut(),
        )?;
        let params = signature
            .ok_or_else(|| windows::core::Error::from_hresult(windows::Win32::Foundation::E_FAIL))?
            .SpawnInstance(0)?;
        Ok(Wmi {
            services,
            path,
            params,
        })
    }
}

impl Wmi {
    fn apply(&self, level: u8) -> windows::core::Result<()> {
        // SAFETY: filling the method's input object and calling it on the live connection.
        unsafe {
            self.params
                .Put(windows::core::w!("Timeout"), 0, &VARIANT::from(1i32), 0)?;
            self.params
                .Put(windows::core::w!("Brightness"), 0, &VARIANT::from(level), 0)?;
            self.services.ExecMethod(
                &self.path,
                &BSTR::from("WmiSetBrightness"),
                WBEM_GENERIC_FLAG_TYPE(0),
                None,
                &self.params,
                None,
                None,
            )
        }
    }
}
