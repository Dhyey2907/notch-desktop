//! Windows virtual desktops for the Home dashboard's switcher. Windows has no public API to
//! list or switch desktops, so the list comes from the registry Explorer keeps
//! (`…\Explorer\VirtualDesktops`), watched with `RegNotifyChangeKeyValue` (no polling), and
//! switching presses Ctrl+Win+Left/Right — the same shortcut you would.
//!
//! The registry's `CurrentVirtualDesktop` isn't updated on every switch, so which desktop is
//! current comes from the documented `IVirtualDesktopManager`: the desktop that the visible
//! windows on screen belong to.

use windows::{
    core::{GUID, HSTRING},
    Win32::{
        Foundation::ERROR_SUCCESS,
        System::Registry::{
            RegGetValueW, RegNotifyChangeKeyValue, RegOpenKeyExW, HKEY, HKEY_CURRENT_USER, KEY_NOTIFY,
            KEY_READ, REG_NOTIFY_CHANGE_LAST_SET, REG_NOTIFY_CHANGE_NAME, RRF_RT_REG_BINARY, RRF_RT_REG_SZ,
        },
        UI::Input::KeyboardAndMouse::{
            SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYBD_EVENT_FLAGS, KEYEVENTF_EXTENDEDKEY,
            KEYEVENTF_KEYUP, VIRTUAL_KEY, VK_CONTROL, VK_LEFT, VK_LWIN, VK_RIGHT,
        },
    },
};

use super::{Desktops, VirtualDesktop};

const KEY: &str = "Software\\Microsoft\\Windows\\CurrentVersion\\Explorer\\VirtualDesktops";
/// Switching animates; give Windows a moment between presses.
const STEP_DELAY: std::time::Duration = std::time::Duration::from_millis(80);

fn read_binary(subkey: &str, value: &str) -> Option<Vec<u8>> {
    let (subkey, value) = (HSTRING::from(subkey), HSTRING::from(value));
    // SAFETY: size query, then a read into a buffer of that size.
    unsafe {
        let mut size = 0u32;
        if RegGetValueW(
            HKEY_CURRENT_USER,
            &subkey,
            &value,
            RRF_RT_REG_BINARY,
            None,
            None,
            Some(&mut size),
        ) != ERROR_SUCCESS
        {
            return None;
        }
        let mut buf = vec![0u8; size as usize];
        (RegGetValueW(
            HKEY_CURRENT_USER,
            &subkey,
            &value,
            RRF_RT_REG_BINARY,
            None,
            Some(buf.as_mut_ptr().cast()),
            Some(&mut size),
        ) == ERROR_SUCCESS)
            .then(|| {
                buf.truncate(size as usize);
                buf
            })
    }
}

fn read_string(subkey: &str, value: &str) -> Option<String> {
    let (subkey, value) = (HSTRING::from(subkey), HSTRING::from(value));
    // SAFETY: as above, for a string value.
    unsafe {
        let mut size = 0u32;
        if RegGetValueW(
            HKEY_CURRENT_USER,
            &subkey,
            &value,
            RRF_RT_REG_SZ,
            None,
            None,
            Some(&mut size),
        ) != ERROR_SUCCESS
        {
            return None;
        }
        let mut buf = vec![0u16; (size as usize).div_ceil(2)];
        (RegGetValueW(
            HKEY_CURRENT_USER,
            &subkey,
            &value,
            RRF_RT_REG_SZ,
            None,
            Some(buf.as_mut_ptr().cast()),
            Some(&mut size),
        ) == ERROR_SUCCESS)
            .then(|| String::from_utf16_lossy(&buf).trim_end_matches('\0').to_owned())
    }
}

/// GUIDs packed 16 bytes each (as Windows stores them).
pub fn guids(bytes: &[u8]) -> Vec<GUID> {
    bytes
        .chunks_exact(16)
        .map(|c| {
            GUID::from_values(
                u32::from_le_bytes([c[0], c[1], c[2], c[3]]),
                u16::from_le_bytes([c[4], c[5]]),
                u16::from_le_bytes([c[6], c[7]]),
                [c[8], c[9], c[10], c[11], c[12], c[13], c[14], c[15]],
            )
        })
        .collect()
}

pub fn read() -> Desktops {
    let ids = read_binary(KEY, "VirtualDesktopIDs")
        .map(|b| guids(&b))
        .unwrap_or_default();
    let current = current_desktop(&ids)
        .or_else(|| read_binary(KEY, "CurrentVirtualDesktop").and_then(|b| guids(&b).first().copied()));
    let desktops = ids
        .iter()
        .enumerate()
        .map(|(i, id)| VirtualDesktop {
            index: i as u32,
            name: read_string(&format!("{KEY}\\Desktops\\{{{id:?}}}"), "Name")
                .filter(|n| !n.trim().is_empty())
                .map(|n| n.chars().take(40).collect())
                .unwrap_or_else(|| format!("Desktop {}", i + 1)),
            current: Some(*id) == current,
        })
        .collect();
    Desktops { desktops }
}

/// Which desktop is on screen: the one the visible windows belong to. On an empty desktop,
/// the one that no visible window belongs to elsewhere (when that leaves a single choice).
fn current_desktop(ids: &[GUID]) -> Option<GUID> {
    use windows::{
        core::BOOL,
        Win32::{
            Foundation::{HWND, LPARAM},
            System::Com::{CoCreateInstance, CoInitializeEx, CLSCTX_ALL, COINIT_MULTITHREADED},
            UI::{
                Shell::{IVirtualDesktopManager, VirtualDesktopManager},
                WindowsAndMessaging::{EnumWindows, IsWindowVisible},
            },
        },
    };
    unsafe extern "system" fn collect(hwnd: HWND, lparam: LPARAM) -> BOOL {
        // SAFETY: `lparam` is the Vec passed below, alive for the whole enumeration.
        let list = unsafe { &mut *(lparam.0 as *mut Vec<HWND>) };
        // SAFETY: a plain query on a window handle from the enumeration.
        if unsafe { IsWindowVisible(hwnd) }.as_bool() {
            list.push(hwnd);
        }
        true.into()
    }
    // SAFETY: COM on this thread (any apartment works for this API); the window list is
    // filled synchronously by EnumWindows before it's read.
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
        let manager: IVirtualDesktopManager =
            CoCreateInstance(&VirtualDesktopManager, None, CLSCTX_ALL).ok()?;
        let mut windows: Vec<HWND> = Vec::new();
        EnumWindows(Some(collect), LPARAM(&mut windows as *mut _ as isize)).ok()?;
        let mut elsewhere = std::collections::HashSet::new();
        for hwnd in windows {
            let Ok(id) = manager.GetWindowDesktopId(hwnd) else {
                continue;
            };
            if !ids.contains(&id) {
                continue; // pinned to all desktops, or not on one
            }
            match manager.IsWindowOnCurrentVirtualDesktop(hwnd) {
                Ok(on) if on.as_bool() => return Some(id),
                Ok(_) => {
                    elsewhere.insert(id);
                }
                Err(_) => {}
            }
        }
        let mut rest = ids.iter().filter(|id| !elsewhere.contains(*id));
        match (rest.next(), rest.next()) {
            (Some(only), None) => Some(*only),
            _ => None,
        }
    }
}

fn key(vk: VIRTUAL_KEY, up: bool) -> INPUT {
    let mut flags = if up { KEYEVENTF_KEYUP } else { KEYBD_EVENT_FLAGS(0) };
    if matches!(vk, VK_LEFT | VK_RIGHT | VK_LWIN) {
        flags |= KEYEVENTF_EXTENDEDKEY;
    }
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: vk,
                dwFlags: flags,
                ..Default::default()
            },
        },
    }
}

/// Go to desktop `target` (0-based) by pressing Ctrl+Win+Left/Right from the current one.
pub fn switch_to(target: u32) -> bool {
    let list = read().desktops;
    let Some(current) = list.iter().find(|d| d.current).map(|d| d.index) else {
        return false;
    };
    if target as usize >= list.len() || target == current {
        return target == current;
    }
    let arrow = if target > current { VK_RIGHT } else { VK_LEFT };
    for _ in 0..target.abs_diff(current) {
        let presses = [
            key(VK_CONTROL, false),
            key(VK_LWIN, false),
            key(arrow, false),
            key(arrow, true),
            key(VK_LWIN, true),
            key(VK_CONTROL, true),
        ];
        // SAFETY: sends a fixed, fully initialised sequence of key events.
        unsafe { SendInput(&presses, std::mem::size_of::<INPUT>() as i32) };
        std::thread::sleep(STEP_DELAY);
    }
    true
}

/// Call `changed` whenever desktops are added, removed, renamed or switched.
pub fn watch(changed: impl Fn() + Send + 'static) {
    std::thread::Builder::new()
        .name("desktops".into())
        .spawn(move || {
            let mut hkey = HKEY::default();
            // SAFETY: opens a key for notifications only; it stays open for the app's life.
            let opened = unsafe {
                RegOpenKeyExW(
                    HKEY_CURRENT_USER,
                    &HSTRING::from(KEY),
                    None,
                    KEY_NOTIFY | KEY_READ,
                    &mut hkey,
                )
            };
            if opened != ERROR_SUCCESS {
                tracing::debug!("virtual desktops key not found");
                return;
            }
            loop {
                // SAFETY: a synchronous wait on our own open key.
                let waited = unsafe {
                    RegNotifyChangeKeyValue(
                        hkey,
                        true,
                        REG_NOTIFY_CHANGE_LAST_SET | REG_NOTIFY_CHANGE_NAME,
                        None,
                        false,
                    )
                };
                if waited != ERROR_SUCCESS {
                    return;
                }
                changed();
            }
        })
        .ok();
}

#[cfg(test)]
mod tests {
    use super::guids;

    #[test]
    fn reads_packed_guids() {
        let bytes = [
            0x78, 0x56, 0x34, 0x12, 0xbc, 0x9a, 0xf0, 0xde, 1, 2, 3, 4, 5, 6, 7, 8, //
            0xff, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        ];
        let ids = guids(&bytes);
        assert_eq!(ids.len(), 2);
        assert_eq!(format!("{:?}", ids[0]), "12345678-9ABC-DEF0-0102-030405060708");
        assert!(guids(&bytes[..20]).len() == 1, "ignores a partial trailing id");
    }
}
