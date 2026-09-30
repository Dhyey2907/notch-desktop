//! Win32 clipboard: a hidden message-only window registered with
//! `AddClipboardFormatListener` (Windows tells it about every copy; nothing polls), reading
//! what was copied, and writing a clip back.

use std::sync::{
    atomic::{AtomicIsize, Ordering},
    OnceLock,
};

use windows::{
    core::{w, PCWSTR, PWSTR},
    Win32::{
        Foundation::{CloseHandle, GlobalFree, HANDLE, HGLOBAL, HWND, LPARAM, LRESULT, POINT, WPARAM},
        System::{
            DataExchange::{
                AddClipboardFormatListener, CloseClipboard, EmptyClipboard, GetClipboardData,
                GetClipboardOwner, IsClipboardFormatAvailable, OpenClipboard, RegisterClipboardFormatW,
                SetClipboardData,
            },
            LibraryLoader::GetModuleHandleW,
            Memory::{GlobalAlloc, GlobalLock, GlobalSize, GlobalUnlock, GMEM_MOVEABLE},
            Ole::{CF_DIB, CF_HDROP, CF_UNICODETEXT},
            Threading::{
                OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32,
                PROCESS_QUERY_LIMITED_INFORMATION,
            },
        },
        UI::{
            Shell::{DragQueryFileW, DROPFILES, HDROP},
            WindowsAndMessaging::{
                CreateWindowExW, DefWindowProcW, DispatchMessageW, GetForegroundWindow, GetMessageW,
                GetWindowThreadProcessId, RegisterClassW, HWND_MESSAGE, MSG, WINDOW_EX_STYLE, WINDOW_STYLE,
                WM_CLIPBOARDUPDATE, WNDCLASSW,
            },
        },
    },
};

use super::MAX_TEXT_BYTES;
use crate::error::{AppError, AppResult};

/// Pictures bigger than this on the clipboard aren't read (a 40 MP bitmap is ~160 MB).
const MAX_IMAGE_BYTES: usize = 200 * 1024 * 1024;

/// What was copied, as read from the clipboard.
pub enum Raw {
    Text(String),
    Png(Vec<u8>),
    Dib(Vec<u8>),
    Files(Vec<String>),
}

pub struct Captured {
    /// The app marked the copy private (password managers do).
    pub private: bool,
    /// Full path of the app that copied it, when Windows says.
    pub exe: Option<String>,
    /// Notch itself copied it (re-copying a clip from the list).
    pub own: bool,
    pub raw: Option<Raw>,
}

type Handler = Box<dyn Fn(Captured) + Send + Sync>;
static HANDLER: OnceLock<Handler> = OnceLock::new();
static WINDOW: AtomicIsize = AtomicIsize::new(0);

/// Start listening on a dedicated thread; `on_copy` runs (on that thread) for every copy.
pub fn listen(on_copy: impl Fn(Captured) + Send + Sync + 'static) {
    if HANDLER.set(Box::new(on_copy)).is_err() {
        return;
    }
    std::thread::Builder::new()
        .name("clipboard".into())
        .spawn(|| {
            if let Err(error) = run() {
                tracing::warn!(%error, "clipboard history unavailable");
            }
        })
        .ok();
}

fn run() -> AppResult<()> {
    let class = w!("NotchClipboardListener");
    // SAFETY: registers a window class whose procedure only reads statics, creates a
    // message-only window owned by this thread, and pumps its messages until the app exits.
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
        WINDOW.store(hwnd.0 as isize, Ordering::SeqCst);
        AddClipboardFormatListener(hwnd)?;
        let mut msg = MSG::default();
        while GetMessageW(&mut msg, None, 0, 0).as_bool() {
            DispatchMessageW(&msg);
        }
    }
    Ok(())
}

unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if msg == WM_CLIPBOARDUPDATE {
        if let Some(handler) = HANDLER.get() {
            handler(capture(hwnd));
        }
        return LRESULT(0);
    }
    // SAFETY: default handling for every other message.
    unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
}

fn format(name: PCWSTR) -> u32 {
    // SAFETY: registering (or looking up) a named clipboard format; `name` is a static string.
    unsafe { RegisterClipboardFormatW(name) }
}

fn available(format: u32) -> bool {
    // SAFETY: a plain query.
    format != 0 && unsafe { IsClipboardFormatAvailable(format) }.is_ok()
}

/// The clipboard is shared: another app may hold it for a moment.
fn open(owner: HWND) -> AppResult<()> {
    for _ in 0..10 {
        // SAFETY: opened here and always closed by the caller.
        if unsafe { OpenClipboard(Some(owner)) }.is_ok() {
            return Ok(());
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    Err(AppError::Platform("the clipboard is busy".into()))
}

fn capture(hwnd: HWND) -> Captured {
    let exe_pid = source_pid();
    let own = exe_pid == Some(std::process::id());
    let private = available(format(w!("ExcludeClipboardContentFromMonitorProcessing")))
        || available(format(w!("Clipboard Viewer Ignore")));
    let mut captured = Captured {
        private,
        exe: exe_pid.and_then(exe_of),
        own,
        raw: None,
    };
    if captured.private || own || open(hwnd).is_err() {
        return captured;
    }
    // SAFETY: the clipboard is open on this thread; handles from GetClipboardData stay
    // owned by the clipboard and are only read while it's open.
    unsafe {
        let history = format(w!("CanIncludeInClipboardHistory"));
        if available(history) {
            let allowed = GetClipboardData(history)
                .ok()
                .and_then(|h| global_bytes(h, 4))
                .and_then(|b| b.get(..4).map(|d| u32::from_le_bytes([d[0], d[1], d[2], d[3]])));
            if allowed == Some(0) {
                captured.private = true;
            }
        }
        if !captured.private {
            captured.raw = read_content();
        }
        let _ = CloseClipboard();
    }
    captured
}

/// Files first (Explorer also offers their names as text), then text, then pictures.
unsafe fn read_content() -> Option<Raw> {
    // SAFETY: called with the clipboard open (see `capture`).
    unsafe {
        if available(u32::from(CF_HDROP.0)) {
            if let Ok(handle) = GetClipboardData(u32::from(CF_HDROP.0)) {
                let files = dropped_files(HDROP(handle.0));
                if !files.is_empty() {
                    return Some(Raw::Files(files));
                }
            }
        }
        if available(u32::from(CF_UNICODETEXT.0)) {
            let bytes = GetClipboardData(u32::from(CF_UNICODETEXT.0))
                .ok()
                .and_then(|h| global_bytes(h, MAX_TEXT_BYTES * 2 + 2))?;
            let units: Vec<u16> = bytes
                .chunks_exact(2)
                .map(|c| u16::from_le_bytes([c[0], c[1]]))
                .take_while(|&u| u != 0)
                .collect();
            return Some(Raw::Text(String::from_utf16_lossy(&units)));
        }
        let png = format(w!("PNG"));
        if available(png) {
            if let Some(bytes) = GetClipboardData(png)
                .ok()
                .and_then(|h| global_bytes(h, MAX_IMAGE_BYTES))
            {
                return Some(Raw::Png(bytes));
            }
        }
        if available(u32::from(CF_DIB.0)) {
            return GetClipboardData(u32::from(CF_DIB.0))
                .ok()
                .and_then(|h| global_bytes(h, MAX_IMAGE_BYTES))
                .map(Raw::Dib);
        }
        None
    }
}

/// Copy a clipboard memory block; `None` if it's empty or bigger than `max`.
unsafe fn global_bytes(handle: HANDLE, max: usize) -> Option<Vec<u8>> {
    let global = HGLOBAL(handle.0);
    // SAFETY: `handle` is a live clipboard HGLOBAL; it's locked only while copied.
    unsafe {
        let size = GlobalSize(global);
        if size == 0 || size > max {
            return None;
        }
        let ptr = GlobalLock(global) as *const u8;
        if ptr.is_null() {
            return None;
        }
        let bytes = std::slice::from_raw_parts(ptr, size).to_vec();
        let _ = GlobalUnlock(global);
        Some(bytes)
    }
}

unsafe fn dropped_files(drop: HDROP) -> Vec<String> {
    // SAFETY: `drop` is the clipboard's CF_HDROP; each buffer is sized from the query.
    unsafe {
        let count = DragQueryFileW(drop, u32::MAX, None);
        (0..count.min(super::MAX_FILES as u32))
            .filter_map(|i| {
                let len = DragQueryFileW(drop, i, None) as usize;
                let mut buf = vec![0u16; len + 1];
                let got = DragQueryFileW(drop, i, Some(&mut buf)) as usize;
                (got > 0).then(|| String::from_utf16_lossy(&buf[..got]))
            })
            .collect()
    }
}

/// The app that owns the clipboard, else the app in front (some apps copy through a
/// helper window that isn't the owner).
fn source_pid() -> Option<u32> {
    // SAFETY: plain queries on window handles owned by the system.
    unsafe {
        let window = GetClipboardOwner()
            .ok()
            .filter(|h| !h.is_invalid())
            .unwrap_or_else(|| GetForegroundWindow());
        let mut pid = 0;
        GetWindowThreadProcessId(window, Some(&mut pid));
        (pid != 0).then_some(pid)
    }
}

fn exe_of(pid: u32) -> Option<String> {
    // SAFETY: a limited query handle to read the image path, closed before returning.
    unsafe {
        let process = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
        let mut buf = [0u16; 1024];
        let mut len = buf.len() as u32;
        let ok = QueryFullProcessImageNameW(process, PROCESS_NAME_WIN32, PWSTR(buf.as_mut_ptr()), &mut len);
        let _ = CloseHandle(process);
        ok.ok()?;
        Some(String::from_utf16_lossy(&buf[..len as usize]))
    }
}

/// What to put back on the clipboard.
pub enum Put<'a> {
    Text(&'a str),
    Image(&'a [u8]),
    Files(&'a [String]),
}

pub fn write(put: Put<'_>) -> AppResult<()> {
    let owner = HWND(WINDOW.load(Ordering::SeqCst) as *mut _);
    if owner.is_invalid() {
        return Err(AppError::Platform("clipboard history isn't running".into()));
    }
    open(owner)?;
    // SAFETY: the clipboard is open on this thread and closed below; each block handed to
    // SetClipboardData becomes the clipboard's (freed by us only if the hand-off fails).
    let result = unsafe {
        EmptyClipboard().map_err(AppError::from).and_then(|()| match put {
            Put::Text(text) => {
                let mut bytes: Vec<u8> = text.encode_utf16().flat_map(u16::to_le_bytes).collect();
                bytes.extend_from_slice(&[0, 0]);
                set(u32::from(CF_UNICODETEXT.0), &bytes)
            }
            Put::Image(png) => {
                set(format(w!("PNG")), png)?;
                match super::image::png_to_dib(png) {
                    Some(dib) => set(u32::from(CF_DIB.0), &dib),
                    None => Ok(()),
                }
            }
            Put::Files(files) => {
                set(u32::from(CF_HDROP.0), &drop_files(files))?;
                // DROPEFFECT_COPY: pasting copies the files rather than moving them.
                set(format(w!("Preferred DropEffect")), &1u32.to_le_bytes())
            }
        })
    };
    // SAFETY: closing what `open` opened.
    unsafe {
        let _ = CloseClipboard();
    }
    result
}

/// `CF_HDROP` data: a DROPFILES header, then each path (UTF-16, NUL-terminated), then a NUL.
fn drop_files(files: &[String]) -> Vec<u8> {
    let header = DROPFILES {
        pFiles: std::mem::size_of::<DROPFILES>() as u32,
        pt: POINT::default(),
        fNC: false.into(),
        fWide: true.into(),
    };
    // SAFETY: DROPFILES is plain old data; viewing it as bytes is sound.
    let mut bytes = unsafe {
        std::slice::from_raw_parts(
            (&header as *const DROPFILES).cast::<u8>(),
            std::mem::size_of::<DROPFILES>(),
        )
    }
    .to_vec();
    for file in files {
        bytes.extend(file.encode_utf16().chain([0]).flat_map(u16::to_le_bytes));
    }
    bytes.extend_from_slice(&[0, 0]);
    bytes
}

unsafe fn set(format: u32, bytes: &[u8]) -> AppResult<()> {
    // SAFETY: a fresh movable block, filled while locked, then handed to the clipboard.
    unsafe {
        let global = GlobalAlloc(GMEM_MOVEABLE, bytes.len().max(1))?;
        let ptr = GlobalLock(global) as *mut u8;
        if ptr.is_null() {
            let _ = GlobalFree(Some(global));
            return Err(AppError::Platform("out of memory".into()));
        }
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), ptr, bytes.len());
        let _ = GlobalUnlock(global);
        if let Err(error) = SetClipboardData(format, Some(HANDLE(global.0))) {
            let _ = GlobalFree(Some(global));
            return Err(error.into());
        }
    }
    Ok(())
}
