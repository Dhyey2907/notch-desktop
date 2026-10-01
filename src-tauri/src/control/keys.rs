//! Caps Lock, Num Lock and the keyboard language of the app in front. Only the lock keys'
//! on/off state and the layout are read — never which keys are typed.

use windows::Win32::{
    Globalization::{GetLocaleInfoEx, LCIDToLocaleName, LOCALE_SLOCALIZEDDISPLAYNAME},
    UI::{
        Input::KeyboardAndMouse::{GetKeyState, GetKeyboardLayout, VK_CAPITAL, VK_NUMLOCK},
        WindowsAndMessaging::{GetForegroundWindow, GetWindowThreadProcessId},
    },
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Keys {
    pub caps: bool,
    pub num: bool,
    /// Language id of the front app's keyboard layout.
    pub language: u16,
    /// The front window, so switching apps isn't mistaken for switching language.
    pub front: isize,
}

pub fn read() -> Keys {
    // SAFETY: plain queries; the toggle bit of each lock key is its on/off state.
    unsafe {
        let mut pid = 0;
        let front = GetForegroundWindow();
        let thread = GetWindowThreadProcessId(front, Some(&mut pid));
        Keys {
            caps: GetKeyState(VK_CAPITAL.0 as i32) & 1 != 0,
            num: GetKeyState(VK_NUMLOCK.0 as i32) & 1 != 0,
            language: (GetKeyboardLayout(thread).0 as usize & 0xFFFF) as u16,
            front: front.0 as isize,
        }
    }
}

/// "English (United States)" for a language id.
pub fn language_name(language: u16) -> Option<String> {
    let mut locale = [0u16; 85];
    // SAFETY: buffers are locals of the documented maximum sizes.
    unsafe {
        let len = LCIDToLocaleName(u32::from(language), Some(&mut locale), 0);
        if len <= 0 {
            return None;
        }
        let mut name = [0u16; 128];
        let len = GetLocaleInfoEx(
            windows::core::PCWSTR(locale.as_ptr()),
            LOCALE_SLOCALIZEDDISPLAYNAME,
            Some(&mut name),
        );
        (len > 1).then(|| String::from_utf16_lossy(&name[..len as usize - 1]))
    }
}
