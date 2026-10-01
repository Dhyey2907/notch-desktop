//! The keyboard hook behind snippets. It turns key presses into characters only to keep the
//! last few in memory (see `Typed`); when a trigger matches, a worker thread checks the field
//! isn't a password box, erases the trigger and types the text.

use std::sync::{
    atomic::{AtomicBool, AtomicIsize, Ordering},
    mpsc::{channel, Sender},
    Mutex, OnceLock, RwLock,
};

use windows::Win32::{
    Foundation::{LPARAM, LRESULT, WPARAM},
    System::{
        Com::{CoCreateInstance, CoInitializeEx, CLSCTX_INPROC_SERVER, COINIT_MULTITHREADED},
        LibraryLoader::GetModuleHandleW,
    },
    UI::{
        Accessibility::{CUIAutomation, IUIAutomation},
        Input::KeyboardAndMouse::{
            GetAsyncKeyState, GetKeyState, GetKeyboardLayout, SendInput, ToUnicodeEx, INPUT, INPUT_0,
            INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP, KEYEVENTF_UNICODE, VIRTUAL_KEY, VK_BACK, VK_CAPITAL,
            VK_CONTROL, VK_LWIN, VK_MENU, VK_RETURN, VK_RWIN, VK_SHIFT,
        },
        WindowsAndMessaging::{
            CallNextHookEx, GetForegroundWindow, GetMessageW, GetWindowThreadProcessId, SetWindowsHookExW,
            HC_ACTION, KBDLLHOOKSTRUCT, LLKHF_INJECTED, MSG, WH_KEYBOARD_LL, WM_KEYDOWN, WM_SYSKEYDOWN,
        },
    },
};

use super::Typed;

pub static SNIPPETS: RwLock<Vec<(String, String)>> = RwLock::new(Vec::new());
pub static ENABLED: AtomicBool = AtomicBool::new(false);
static TYPED: Mutex<Typed> = Mutex::new(Typed { chars: Vec::new() });
static FRONT: AtomicIsize = AtomicIsize::new(0);
static WORKER: OnceLock<Sender<(usize, String)>> = OnceLock::new();
static STARTED: AtomicBool = AtomicBool::new(false);

/// Keys that move the caret or leave the field: what was typed before no longer counts.
fn resets(vk: u32) -> bool {
    matches!(vk, 0x09 | 0x0D | 0x1B | 0x21..=0x28 | 0x2E)
}

fn pressed(vk: VIRTUAL_KEY) -> bool {
    // SAFETY: plain query.
    unsafe { GetAsyncKeyState(i32::from(vk.0)) < 0 }
}

unsafe extern "system" fn on_key(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code == HC_ACTION as i32
        && ENABLED.load(Ordering::Relaxed)
        && (wparam.0 == WM_KEYDOWN as usize || wparam.0 == WM_SYSKEYDOWN as usize)
    {
        // SAFETY: for WH_KEYBOARD_LL, lparam points to a KBDLLHOOKSTRUCT.
        let key = unsafe { &*(lparam.0 as *const KBDLLHOOKSTRUCT) };
        if key.flags.0 & LLKHF_INJECTED.0 == 0 {
            seen(key.vkCode, key.scanCode);
        }
    }
    // SAFETY: passing the event on unchanged.
    unsafe { CallNextHookEx(None, code, wparam, lparam) }
}

fn seen(vk: u32, scan: u32) {
    // SAFETY: plain queries on the front window.
    let (front, layout, own) = unsafe {
        let hwnd = GetForegroundWindow();
        let mut pid = 0;
        let thread = GetWindowThreadProcessId(hwnd, Some(&mut pid));
        (
            hwnd.0 as isize,
            GetKeyboardLayout(thread),
            pid == std::process::id(),
        )
    };
    let mut typed = TYPED.lock().unwrap_or_else(|e| e.into_inner());
    // Never in Notch itself (e.g. while writing a snippet in Settings).
    if own {
        typed.clear();
        return;
    }
    if FRONT.swap(front, Ordering::Relaxed) != front {
        typed.clear();
    }
    if vk == u32::from(VK_BACK.0) {
        typed.backspace();
        return;
    }
    if resets(vk) || pressed(VK_CONTROL) || pressed(VK_LWIN) || pressed(VK_RWIN) {
        typed.clear();
        return;
    }
    // Build the modifier state ourselves: this thread's own keyboard state is stale.
    let mut state = [0u8; 256];
    if pressed(VK_SHIFT) {
        state[usize::from(VK_SHIFT.0)] = 0x80;
    }
    // AltGr arrives as Ctrl+Alt, which is handled above; plain Alt doesn't type.
    if pressed(VK_MENU) {
        typed.clear();
        return;
    }
    // SAFETY: plain query; bit 0 is the toggle.
    if unsafe { GetKeyState(i32::from(VK_CAPITAL.0)) } & 1 != 0 {
        state[usize::from(VK_CAPITAL.0)] = 0x01;
    }
    let mut out = [0u16; 4];
    // SAFETY: local buffers; flag 4 leaves the keyboard's dead-key state alone.
    let n = unsafe { ToUnicodeEx(vk, scan, &state, &mut out, 4, Some(layout)) };
    if n <= 0 {
        return;
    }
    for c in char::decode_utf16(out[..n as usize].iter().copied()).flatten() {
        if !c.is_control() {
            typed.push(c);
        }
    }
    let snippets = SNIPPETS.read().unwrap_or_else(|e| e.into_inner());
    if let Some((trigger, body)) = typed.matched(&snippets) {
        let erase = trigger.chars().count();
        typed.clear();
        if let Some(worker) = WORKER.get() {
            let _ = worker.send((erase, body.clone()));
        }
    }
}

/// Is keyboard focus in a password box? (Then nothing is expanded.)
fn in_password_field(automation: Option<&IUIAutomation>) -> bool {
    let Some(automation) = automation else {
        return false;
    };
    // SAFETY: UI Automation calls on this worker's MTA.
    unsafe {
        automation
            .GetFocusedElement()
            .and_then(|e| e.CurrentIsPassword())
            .is_ok_and(|p| p.as_bool())
    }
}

fn key(vk: VIRTUAL_KEY, unicode: u16, up: bool) -> INPUT {
    let mut flags = if unicode != 0 {
        KEYEVENTF_UNICODE
    } else {
        Default::default()
    };
    if up {
        flags |= KEYEVENTF_KEYUP;
    }
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: vk,
                wScan: unicode,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    }
}

fn type_text(erase: usize, text: &str) {
    let mut inputs = Vec::new();
    for _ in 0..erase {
        inputs.push(key(VK_BACK, 0, false));
        inputs.push(key(VK_BACK, 0, true));
    }
    for unit in text.replace("\r\n", "\n").encode_utf16() {
        if unit == u16::from(b'\n') {
            inputs.push(key(VK_RETURN, 0, false));
            inputs.push(key(VK_RETURN, 0, true));
        } else {
            inputs.push(key(VIRTUAL_KEY(0), unit, false));
            inputs.push(key(VIRTUAL_KEY(0), unit, true));
        }
    }
    // SAFETY: a slice of well-formed keyboard inputs.
    unsafe {
        SendInput(&inputs, std::mem::size_of::<INPUT>() as i32);
    }
}

/// Install the hook (once). It does nothing while `ENABLED` is false.
pub fn start() {
    if STARTED.swap(true, Ordering::SeqCst) {
        return;
    }
    let (tx, rx) = channel::<(usize, String)>();
    let _ = WORKER.set(tx);
    std::thread::Builder::new()
        .name("snippets-typer".into())
        .spawn(move || {
            // SAFETY: COM for UI Automation on this thread.
            let automation: Option<IUIAutomation> = unsafe {
                let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
                CoCreateInstance(&CUIAutomation, None, CLSCTX_INPROC_SERVER).ok()
            };
            for (erase, body) in rx {
                if in_password_field(automation.as_ref()) {
                    continue;
                }
                // Let the trigger's last key finish reaching the app first.
                std::thread::sleep(std::time::Duration::from_millis(30));
                type_text(erase, &super::fill(&body, chrono::Local::now()));
            }
        })
        .ok();
    std::thread::Builder::new()
        .name("snippets-hook".into())
        .spawn(|| {
            // SAFETY: a low-level hook needs a message loop on the installing thread.
            unsafe {
                let module = GetModuleHandleW(None).ok();
                let hook = SetWindowsHookExW(WH_KEYBOARD_LL, Some(on_key), module.map(Into::into), 0);
                if hook.is_err() {
                    tracing::warn!("could not start snippets");
                    return;
                }
                let mut msg = MSG::default();
                while GetMessageW(&mut msg, None, 0, 0).as_bool() {}
            }
        })
        .ok();
}
