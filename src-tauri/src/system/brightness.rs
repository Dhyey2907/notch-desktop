//! Laptop screen brightness: Windows sends a power-setting notification with the new level
//! whenever it changes (brightness keys, the quick settings slider, auto-brightness). External
//! monitors don't report brightness, so this covers the built-in screen only.

use std::sync::OnceLock;

use windows::{
    core::w,
    Win32::{
        Foundation::{HANDLE, HWND, LPARAM, LRESULT, WPARAM},
        System::{
            LibraryLoader::GetModuleHandleW,
            Power::{RegisterPowerSettingNotification, POWERBROADCAST_SETTING},
            SystemServices::GUID_VIDEO_CURRENT_MONITOR_BRIGHTNESS,
        },
        UI::WindowsAndMessaging::{
            CreateWindowExW, DefWindowProcW, DispatchMessageW, GetMessageW, RegisterClassW,
            DEVICE_NOTIFY_WINDOW_HANDLE, HWND_MESSAGE, MSG, PBT_POWERSETTINGCHANGE, WINDOW_EX_STYLE,
            WINDOW_STYLE, WM_POWERBROADCAST, WNDCLASSW,
        },
    },
};

type Handler = Box<dyn Fn(u8, bool) + Send + Sync>;
static HANDLER: OnceLock<Handler> = OnceLock::new();
static FIRST: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(true);

/// Start watching; `report(percent, initial)` runs on a dedicated thread. Windows sends the
/// current level once right after registering (`initial`), then on every change.
pub fn start(report: impl Fn(u8, bool) + Send + Sync + 'static) {
    if HANDLER.set(Box::new(report)).is_err() {
        return;
    }
    std::thread::Builder::new()
        .name("brightness".into())
        .spawn(|| {
            if let Err(error) = run() {
                tracing::warn!(%error, "brightness display unavailable");
            }
        })
        .ok();
}

fn run() -> windows::core::Result<()> {
    let class = w!("NotchBrightnessListener");
    // SAFETY: a message-only window owned by this thread, whose procedure only reads statics,
    // registered for one power setting and pumped until the app exits.
    unsafe {
        let instance = GetModuleHandleW(None)?;
        let wc = WNDCLASSW {
            lpfnWndProc: Some(wndproc),
            hInstance: instance.into(),
            lpszClassName: class,
            ..Default::default()
        };
        RegisterClassW(&wc);
        let hwnd = CreateWindowExW(
            WINDOW_EX_STYLE(0),
            class,
            w!(""),
            WINDOW_STYLE(0),
            0,
            0,
            0,
            0,
            Some(HWND_MESSAGE),
            None,
            Some(instance.into()),
            None,
        )?;
        RegisterPowerSettingNotification(
            HANDLE(hwnd.0),
            &GUID_VIDEO_CURRENT_MONITOR_BRIGHTNESS,
            DEVICE_NOTIFY_WINDOW_HANDLE,
        )?;
        let mut msg = MSG::default();
        while GetMessageW(&mut msg, None, 0, 0).as_bool() {
            DispatchMessageW(&msg);
        }
    }
    Ok(())
}

unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if msg == WM_POWERBROADCAST && wparam.0 == PBT_POWERSETTINGCHANGE as usize {
        // SAFETY: for PBT_POWERSETTINGCHANGE, lparam points to a POWERBROADCAST_SETTING whose
        // data (a DWORD percentage for this setting) follows the header.
        let level = unsafe {
            let setting = &*(lparam.0 as *const POWERBROADCAST_SETTING);
            (setting.PowerSetting == GUID_VIDEO_CURRENT_MONITOR_BRIGHTNESS && setting.DataLength >= 4)
                .then(|| std::ptr::read_unaligned(setting.Data.as_ptr().cast::<u32>()))
        };
        if let (Some(level), Some(handler)) = (level, HANDLER.get()) {
            let initial = FIRST.swap(false, std::sync::atomic::Ordering::SeqCst);
            handler(level.min(100) as u8, initial);
        }
        return LRESULT(1);
    }
    // SAFETY: default handling for every other message.
    unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
}
