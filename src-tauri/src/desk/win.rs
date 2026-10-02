//! Windows calls for Desk: list windows, pin on top, see-through, bring to front, close an
//! app's windows politely, and DWM thumbnails (a live view of a window drawn into the notch).

use windows::core::BOOL;
use windows::Win32::{
    Foundation::{COLORREF, HWND, LPARAM, RECT, WPARAM},
    Graphics::Dwm::{
        DwmGetWindowAttribute, DwmQueryThumbnailSourceSize, DwmRegisterThumbnail, DwmUnregisterThumbnail,
        DwmUpdateThumbnailProperties, DWMWA_CLOAKED, DWM_THUMBNAIL_PROPERTIES, DWM_TNP_OPACITY,
        DWM_TNP_RECTDESTINATION, DWM_TNP_SOURCECLIENTAREAONLY, DWM_TNP_VISIBLE,
    },
    UI::WindowsAndMessaging::{
        EnumWindows, GetLayeredWindowAttributes, GetWindow, GetWindowLongPtrW, GetWindowTextLengthW,
        GetWindowTextW, GetWindowThreadProcessId, IsIconic, IsWindow, IsWindowVisible, PostMessageW,
        SetForegroundWindow, SetLayeredWindowAttributes, SetWindowLongPtrW, SetWindowPos, ShowWindow,
        GWL_EXSTYLE, GW_OWNER, HWND_NOTOPMOST, HWND_TOPMOST, LAYERED_WINDOW_ATTRIBUTES_FLAGS, LWA_ALPHA,
        SWP_NOMOVE, SWP_NOSIZE, SW_RESTORE, WM_CLOSE, WS_EX_LAYERED, WS_EX_TOOLWINDOW, WS_EX_TOPMOST,
    },
};

use super::DeskWindow;
use crate::error::{AppError, AppResult};

fn hwnd(id: i64) -> HWND {
    HWND(id as isize as *mut _)
}

fn title_of(h: HWND) -> String {
    // SAFETY: reads the title into a local buffer of the reported length.
    unsafe {
        let len = GetWindowTextLengthW(h);
        if len <= 0 {
            return String::new();
        }
        let mut buf = vec![0u16; len as usize + 1];
        let n = GetWindowTextW(h, &mut buf);
        String::from_utf16_lossy(&buf[..n as usize])
    }
}

fn pid_of(h: HWND) -> u32 {
    let mut pid = 0;
    // SAFETY: plain query.
    unsafe { GetWindowThreadProcessId(h, Some(&mut pid)) };
    pid
}

/// Windows you see in Alt+Tab: visible, titled, not owned, not tool windows, not cloaked.
pub fn list() -> Vec<DeskWindow> {
    let mut handles: Vec<HWND> = Vec::new();
    unsafe extern "system" fn collect(h: HWND, data: LPARAM) -> BOOL {
        // SAFETY: `data` is the Vec passed below, alive for the whole EnumWindows call.
        let handles = unsafe { &mut *(data.0 as *mut Vec<HWND>) };
        handles.push(h);
        true.into()
    }
    // SAFETY: the callback only pushes into the Vec it's given.
    unsafe {
        let _ = EnumWindows(Some(collect), LPARAM(&mut handles as *mut _ as isize));
    }
    let own = std::process::id();
    handles
        .into_iter()
        .filter_map(|h| {
            // SAFETY: read-only queries on a window handle from EnumWindows.
            let (visible, owned, ex, cloaked) = unsafe {
                let mut cloaked = 0u32;
                let _ = DwmGetWindowAttribute(
                    h,
                    DWMWA_CLOAKED,
                    (&mut cloaked as *mut u32).cast(),
                    std::mem::size_of::<u32>() as u32,
                );
                (
                    IsWindowVisible(h).as_bool(),
                    GetWindow(h, GW_OWNER).is_ok_and(|o| !o.is_invalid()),
                    GetWindowLongPtrW(h, GWL_EXSTYLE) as u32,
                    cloaked != 0,
                )
            };
            if !visible || owned || cloaked || ex & WS_EX_TOOLWINDOW.0 != 0 {
                return None;
            }
            let title = title_of(h);
            let pid = pid_of(h);
            if title.trim().is_empty() || pid == own {
                return None;
            }
            let exe = crate::clipboard::win::exe_of(pid)?;
            let app = crate::clipboard::app_label(&exe);
            if ["explorer.exe", "textinputhost.exe", "applicationframehost.exe"]
                .contains(&crate::clipboard::exe_key(&exe).as_str())
                && title == "Program Manager"
            {
                return None;
            }
            Some(DeskWindow {
                id: h.0 as isize as i64,
                title,
                app,
                on_top: ex & WS_EX_TOPMOST.0 != 0,
                opacity: opacity(h),
            })
        })
        .take(60)
        .collect()
}

fn opacity(h: HWND) -> u8 {
    // SAFETY: read-only queries.
    unsafe {
        let ex = GetWindowLongPtrW(h, GWL_EXSTYLE) as u32;
        if ex & WS_EX_LAYERED.0 == 0 {
            return 100;
        }
        let mut alpha = 255u8;
        let mut flags = LAYERED_WINDOW_ATTRIBUTES_FLAGS(0);
        if GetLayeredWindowAttributes(h, None, Some(&mut alpha), Some(&mut flags)).is_ok()
            && flags.0 & LWA_ALPHA.0 != 0
        {
            ((u32::from(alpha) * 100 + 127) / 255) as u8
        } else {
            100
        }
    }
}

fn check(id: i64) -> AppResult<HWND> {
    let h = hwnd(id);
    // SAFETY: plain query.
    if unsafe { !IsWindow(Some(h)).as_bool() } {
        return Err(AppError::InvalidInput("That window was closed".into()));
    }
    if pid_of(h) == std::process::id() {
        return Err(AppError::InvalidInput("That's Notch itself".into()));
    }
    Ok(h)
}

/// Windows stops apps from changing windows that run as administrator.
fn denied() -> AppError {
    AppError::InvalidInput("Windows didn't allow that — the app may be running as administrator".into())
}

pub fn set_on_top(id: i64, on: bool) -> AppResult<()> {
    let h = check(id)?;
    // SAFETY: changing z-order only (no move, no resize).
    unsafe {
        SetWindowPos(
            h,
            Some(if on { HWND_TOPMOST } else { HWND_NOTOPMOST }),
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE,
        )
        .map_err(|_| denied())
    }
}

pub fn set_opacity(id: i64, percent: u8) -> AppResult<()> {
    let h = check(id)?;
    let percent = percent.clamp(20, 100);
    // SAFETY: toggles WS_EX_LAYERED and sets the window's alpha.
    unsafe {
        let ex = GetWindowLongPtrW(h, GWL_EXSTYLE);
        if percent >= 100 {
            if ex as u32 & WS_EX_LAYERED.0 != 0 {
                SetWindowLongPtrW(h, GWL_EXSTYLE, ex & !(WS_EX_LAYERED.0 as isize));
            }
            return Ok(());
        }
        SetWindowLongPtrW(h, GWL_EXSTYLE, ex | WS_EX_LAYERED.0 as isize);
        let alpha = (u32::from(percent) * 255 / 100) as u8;
        SetLayeredWindowAttributes(h, COLORREF(0), alpha, LWA_ALPHA).map_err(|_| denied())
    }
}

pub fn focus(id: i64) -> AppResult<()> {
    let h = check(id)?;
    // SAFETY: restore if minimized, then bring to front.
    unsafe {
        if IsIconic(h).as_bool() {
            let _ = ShowWindow(h, SW_RESTORE);
        }
        let _ = SetForegroundWindow(h);
    }
    Ok(())
}

/// The window in front right now.
pub fn front() -> i64 {
    // SAFETY: plain query.
    unsafe { windows::Win32::UI::WindowsAndMessaging::GetForegroundWindow().0 as isize as i64 }
}

/// Ask every window of a program to close (like clicking ×; apps can still ask to save).
pub fn close_app(exe_key: &str) -> usize {
    let mut closed = 0;
    for w in list_all_of(exe_key) {
        // SAFETY: posting WM_CLOSE to a top-level window.
        if unsafe { PostMessageW(Some(w), WM_CLOSE, WPARAM(0), LPARAM(0)) }.is_ok() {
            closed += 1;
        }
    }
    closed
}

fn list_all_of(exe_key: &str) -> Vec<HWND> {
    list()
        .into_iter()
        .map(|w| hwnd(w.id))
        .filter(|h| {
            crate::clipboard::win::exe_of(pid_of(*h))
                .is_some_and(|exe| crate::clipboard::exe_key(&exe) == exe_key)
        })
        .collect()
}

/// A live view of `source` drawn into `dest` (our island) at `rect` (physical pixels).
pub fn thumbnail(dest: HWND, source: i64, rect: (i32, i32, i32, i32)) -> AppResult<isize> {
    let source = check(source)?;
    // SAFETY: registers a DWM thumbnail between two live top-level windows.
    let id = unsafe { DwmRegisterThumbnail(dest, source) }.map_err(|e| AppError::Platform(e.to_string()))?;
    move_thumbnail(id, rect)?;
    Ok(id)
}

/// The largest rectangle with the source's shape that fits in `area`, centered.
pub fn fit((left, top, right, bottom): (i32, i32, i32, i32), source: (i32, i32)) -> (i32, i32, i32, i32) {
    let (w, h) = (right - left, bottom - top);
    if source.0 <= 0 || source.1 <= 0 || w <= 0 || h <= 0 {
        return (left, top, right, bottom);
    }
    let scale = (f64::from(w) / f64::from(source.0)).min(f64::from(h) / f64::from(source.1));
    let (fw, fh) = (
        (f64::from(source.0) * scale).round() as i32,
        (f64::from(source.1) * scale).round() as i32,
    );
    let (x, y) = (left + (w - fw) / 2, top + (h - fh) / 2);
    (x, y, x + fw, y + fh)
}

pub fn move_thumbnail(id: isize, area: (i32, i32, i32, i32)) -> AppResult<()> {
    // SAFETY: querying a thumbnail we registered.
    let source = unsafe { DwmQueryThumbnailSourceSize(id) }
        .map(|s| (s.cx, s.cy))
        .unwrap_or((0, 0));
    let (left, top, right, bottom) = fit(area, source);
    let props = DWM_THUMBNAIL_PROPERTIES {
        dwFlags: DWM_TNP_RECTDESTINATION | DWM_TNP_VISIBLE | DWM_TNP_OPACITY | DWM_TNP_SOURCECLIENTAREAONLY,
        rcDestination: RECT {
            left,
            top,
            right,
            bottom,
        },
        opacity: 255,
        fVisible: true.into(),
        fSourceClientAreaOnly: false.into(),
        ..Default::default()
    };
    // SAFETY: updating a thumbnail we registered.
    unsafe { DwmUpdateThumbnailProperties(id, &props) }.map_err(|e| AppError::Platform(e.to_string()))
}

pub fn drop_thumbnail(id: isize) {
    // SAFETY: unregistering a thumbnail we registered.
    unsafe {
        let _ = DwmUnregisterThumbnail(id);
    }
}
