//! Desktop shell: island window placement, tray, global shortcut, and window lifecycle
//! (architecture.md §3–4). The frontend decides *what size* the island needs; this
//! module decides *where* that lands on the display.

pub mod islands;
mod placement;

use std::{collections::HashMap, sync::Mutex};

use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Manager, WebviewUrl, WebviewWindow, WebviewWindowBuilder, WindowEvent,
};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};

use crate::{error::AppResult, events::AppEvent, state::AppState};
pub use islands::ShellState;
use placement::{top_center_frame, WorkArea};

pub const MAIN_WINDOW: &str = "main";
pub const SETTINGS_WINDOW: &str = "settings";
/// Passed by the Windows startup entry so a sign-in launch can be told apart from a manual one.
pub const AUTOSTART_ARG: &str = "--autostart";

/// Open the settings window, or focus it if it is already open. It is an ordinary
/// decorated window, created on demand and destroyed on close (design.md §21).
pub fn open_settings_window(app: &AppHandle, section: Option<&str>) -> AppResult<()> {
    // Section ids end up in a URL fragment and a script; allow only lowercase ASCII letters.
    let section =
        section.filter(|s| !s.is_empty() && s.len() <= 32 && s.chars().all(|c| c.is_ascii_lowercase()));
    let hash = match section {
        Some(section) => format!("#settings/{section}"),
        None => "#settings".to_owned(),
    };
    if let Some(window) = app.get_webview_window(SETTINGS_WINDOW) {
        if section.is_some() {
            window.eval(format!("window.location.hash = '{hash}'"))?;
        }
        window.unminimize()?;
        window.show()?;
        window.set_focus()?;
        return Ok(());
    }
    WebviewWindowBuilder::new(
        app,
        SETTINGS_WINDOW,
        WebviewUrl::App(format!("index.html{hash}").into()),
    )
    .title("Notch Desktop Settings")
    .inner_size(760.0, 540.0)
    .min_inner_size(640.0, 440.0)
    .center()
    .build()?;
    Ok(())
}

/// Size and position an island top-center on its screen (design.md §20).
pub fn place(window: &WebviewWindow, (width, height): (f64, f64)) -> AppResult<()> {
    let shell = window.app_handle().state::<ShellState>();
    let Some(monitor) = islands::monitor_for(window, &shell) else {
        tracing::warn!("no monitor available; skipping island placement");
        return Ok(());
    };
    let area = monitor.work_area();
    let work = WorkArea {
        x: area.position.x,
        y: area.position.y,
        width: area.size.width,
        height: area.size.height,
    };
    let (position, size) = top_center_frame(work, monitor.scale_factor(), width, height);
    window.set_size(size)?;
    window.set_position(position)?;
    Ok(())
}

/// Show the island and keep it out of Alt+Tab. Use this instead of `window.show()`.
pub fn show_island(window: &WebviewWindow) -> AppResult<()> {
    // Showing an already-visible window makes Tauri re-apply its styles, which would
    // force the hide/show below and steal focus mid-interaction.
    if window.is_visible()? {
        return Ok(());
    }
    window.show()?;
    #[cfg(windows)]
    make_tool_window(window)?;
    Ok(())
}

/// Give the window `WS_EX_TOOLWINDOW` so it has no Alt+Tab entry; Tauri's `skipTaskbar`
/// only removes the taskbar button. Tauri re-applies its own styles on every `show()`,
/// so this runs after each show. Windows only re-reads the Alt+Tab list when a window is
/// shown, hence the raw hide/show (which bypasses Tauri) after changing the style.
#[cfg(windows)]
fn make_tool_window(window: &WebviewWindow) -> AppResult<()> {
    use windows::Win32::UI::WindowsAndMessaging::{
        GetWindowLongPtrW, SetWindowLongPtrW, ShowWindow, GWL_EXSTYLE, SW_HIDE, SW_SHOWNOACTIVATE,
        WS_EX_APPWINDOW, WS_EX_TOOLWINDOW,
    };
    // Tauri may link a different `windows` version; both HWNDs wrap the same raw handle.
    let hwnd = windows::Win32::Foundation::HWND(window.hwnd()?.0);
    // SAFETY: `hwnd` is a live top-level window owned by this process; we only toggle
    // extended-style bits and visibility on it.
    unsafe {
        let style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
        let tool_style = (style | WS_EX_TOOLWINDOW.0 as isize) & !(WS_EX_APPWINDOW.0 as isize);
        if tool_style != style {
            SetWindowLongPtrW(hwnd, GWL_EXSTYLE, tool_style);
            let _ = ShowWindow(hwnd, SW_HIDE);
            let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
        }
    }
    Ok(())
}

/// Tray show/hide acts on every island together.
fn toggle_visibility(app: &AppHandle) {
    let windows = islands::all(app);
    let hide = windows.iter().any(|w| w.is_visible().unwrap_or(false));
    for window in windows {
        let result = if hide {
            window.hide().map_err(Into::into)
        } else {
            show_island(&window)
        };
        if let Err(error) = result {
            tracing::warn!(%error, "failed to toggle island visibility");
        }
    }
}

fn setup_tray(app: &AppHandle) -> AppResult<()> {
    let toggle = MenuItem::with_id(app, "toggle", "Show / hide notch", true, None::<&str>)?;
    let settings = MenuItem::with_id(app, "settings", "Settings…", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit Notch Desktop", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&toggle, &settings, &quit])?;

    let mut tray = TrayIconBuilder::with_id("main")
        .tooltip("Notch Desktop")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "toggle" => toggle_visibility(app),
            "settings" => {
                if let Err(error) = open_settings_window(app, None) {
                    tracing::warn!(%error, "failed to open settings window");
                }
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                toggle_visibility(tray.app_handle());
            }
        });
    if let Some(icon) = app.default_window_icon() {
        tray = tray.icon(icon.clone());
    }
    tray.build(app)?;
    Ok(())
}

/// Ctrl+Alt+N toggles the expanded island from anywhere (keyboard accessibility, design.md §18).
fn toggle_shortcut() -> Shortcut {
    Shortcut::new(Some(Modifiers::CONTROL | Modifiers::ALT), Code::KeyN)
}

pub fn shortcut_plugin() -> tauri::plugin::TauriPlugin<tauri::Wry> {
    let toggle = toggle_shortcut();
    tauri_plugin_global_shortcut::Builder::new()
        .with_handler(move |app, shortcut, event| {
            if shortcut != &toggle || event.state() != ShortcutState::Pressed {
                return;
            }
            // The island on the screen the pointer is on.
            let Some(window) = islands::under_cursor(app) else {
                return;
            };
            let _ = show_island(&window);
            let _ = window.set_focus();
            app.state::<AppState>().bus.emit(
                "shell",
                AppEvent::ShellToggleRequested {
                    label: window.label().to_owned(),
                },
            );
        })
        .build()
}

/// Wire up the tray, shortcut, and window lifecycle, then show the island.
pub fn init(app: &AppHandle) -> AppResult<()> {
    setup_tray(app)?;

    if let Err(error) = app.global_shortcut().register(toggle_shortcut()) {
        // Another app may own the chord; the island still works by pointer and tray.
        tracing::warn!(%error, "could not register the Ctrl+Alt+N shortcut");
    }

    // `main` comes from tauri.conf.json; other screens' islands are created by `sync`.
    if let Some(window) = app.get_webview_window(MAIN_WINDOW) {
        attach_island(app, &window)?;
    }
    islands::sync(app);
    watch_display_setting(app);
    Ok(())
}

/// Per-window setup shared by every island: events, placement, system watchers, show.
fn attach_island(app: &AppHandle, window: &WebviewWindow) -> AppResult<()> {
    let label = window.label().to_owned();
    let handle = app.clone();
    window.on_window_event(move |event| match event {
        // No decorations, but Alt+F4 still arrives: hide to tray instead of quitting.
        WindowEvent::CloseRequested { api, .. } => {
            api.prevent_close();
            if let Some(window) = handle.get_webview_window(&label) {
                let _ = window.hide();
            }
        }
        WindowEvent::ScaleFactorChanged { .. } => {
            if let Some(window) = handle.get_webview_window(&label) {
                let size = handle.state::<ShellState>().window_size(&label);
                if let Err(error) = place(&window, size) {
                    tracing::warn!(%error, "failed to re-place island after a scale change");
                }
            }
        }
        _ => {}
    });
    place(window, app.state::<ShellState>().window_size(window.label()))?;
    #[cfg(windows)]
    watch_system_changes(window);
    show_island(window)?;
    Ok(())
}

/// Re-sync islands when the "Show the notch on" setting changes.
fn watch_display_setting(app: &AppHandle) {
    let app = app.clone();
    let mut events = app.state::<AppState>().bus.subscribe();
    tauri::async_runtime::spawn(async move {
        let mut last = app
            .state::<AppState>()
            .settings
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .display
            .clone();
        loop {
            match events.recv().await {
                Ok(envelope) => {
                    if let AppEvent::SettingsChanged(settings) = &envelope.event {
                        if settings.display != last {
                            last = settings.display.clone();
                            islands::sync(&app);
                        }
                    }
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                Err(_) => break,
            }
        }
    });
}

/// Fullscreen state per island: Windows reports fullscreen apps per monitor.
static FULLSCREEN: Mutex<Option<HashMap<String, bool>>> = Mutex::new(None);

pub fn is_fullscreen(label: &str) -> bool {
    FULLSCREEN
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .as_ref()
        .and_then(|map| map.get(label).copied())
        .unwrap_or(false)
}

fn set_fullscreen(app: &AppHandle, label: &str, active: bool) {
    let changed = {
        let mut guard = FULLSCREEN.lock().unwrap_or_else(|e| e.into_inner());
        guard
            .get_or_insert_with(HashMap::new)
            .insert(label.to_owned(), active)
            != Some(active)
    };
    if changed {
        tracing::debug!(active, "fullscreen app changed");
        app.state::<AppState>().bus.emit(
            "shell",
            AppEvent::FullscreenChanged {
                label: label.to_owned(),
                active,
            },
        );
    }
}

/// Listen for system changes the island reacts to. Tauri doesn't surface these, so the
/// island's window procedure is subclassed:
/// - display layout (resolution, monitors added or removed, primary switched, taskbar moved):
///   re-place the island, coalescing bursts until the layout settles;
/// - fullscreen apps opening or closing: Windows tells registered app bars
///   (`ABN_FULLSCREENAPP`). Notch registers one only for that notice — it never reserves
///   screen space.
#[cfg(windows)]
fn watch_system_changes(window: &WebviewWindow) {
    use std::sync::{
        atomic::{AtomicBool, Ordering},
        OnceLock,
    };
    use windows::Win32::{
        Foundation::{HWND, LPARAM, LRESULT, WPARAM},
        Graphics::Gdi::{MonitorFromWindow, MONITOR_DEFAULTTONEAREST},
        UI::{
            Shell::{
                DefSubclassProc, SHAppBarMessage, SHQueryUserNotificationState, SetWindowSubclass, ABM_NEW,
                ABM_REMOVE, ABN_FULLSCREENAPP, APPBARDATA, QUNS_BUSY, QUNS_PRESENTATION_MODE,
                QUNS_RUNNING_D3D_FULL_SCREEN,
            },
            WindowsAndMessaging::{
                GetForegroundWindow, SPI_SETWORKAREA, WM_APP, WM_DISPLAYCHANGE, WM_NCDESTROY,
                WM_SETTINGCHANGE,
            },
        },
    };

    const SETTLE: std::time::Duration = std::time::Duration::from_millis(400);
    const APPBAR_CALLBACK: u32 = WM_APP + 0x42;
    static APP: OnceLock<AppHandle> = OnceLock::new();
    static PENDING: AtomicBool = AtomicBool::new(false);
    /// Island label for each subclassed window handle.
    static LABELS: Mutex<Option<HashMap<isize, String>>> = Mutex::new(None);

    fn label_of(hwnd: HWND) -> Option<String> {
        LABELS
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .as_ref()
            .and_then(|map| map.get(&(hwnd.0 as isize)).cloned())
    }

    fn appbar(hwnd: HWND) -> APPBARDATA {
        APPBARDATA {
            cbSize: std::mem::size_of::<APPBARDATA>() as u32,
            hWnd: hwnd,
            uCallbackMessage: APPBAR_CALLBACK,
            ..Default::default()
        }
    }

    unsafe extern "system" fn subclass(
        hwnd: HWND,
        msg: u32,
        wparam: WPARAM,
        lparam: LPARAM,
        _id: usize,
        _data: usize,
    ) -> LRESULT {
        let layout_changed =
            msg == WM_DISPLAYCHANGE || (msg == WM_SETTINGCHANGE && wparam.0 == SPI_SETWORKAREA.0 as usize);
        if layout_changed && !PENDING.swap(true, Ordering::SeqCst) {
            if let Some(app) = APP.get().cloned() {
                std::thread::spawn(move || {
                    std::thread::sleep(SETTLE);
                    PENDING.store(false, Ordering::SeqCst);
                    // Screens may have come or gone: open/close islands and re-place them all.
                    islands::sync(&app);
                });
            } else {
                PENDING.store(false, Ordering::SeqCst);
            }
        }
        if msg == APPBAR_CALLBACK && wparam.0 == ABN_FULLSCREENAPP as usize {
            if let (Some(app), Some(label)) = (APP.get().cloned(), label_of(hwnd)) {
                let active = lparam.0 != 0;
                // Off the window procedure: emitting reaches into the webview.
                std::thread::spawn(move || set_fullscreen(&app, &label, active));
            }
        }
        if msg == WM_NCDESTROY {
            let mut data = appbar(hwnd);
            // SAFETY: unregistering the app bar this window registered.
            unsafe { SHAppBarMessage(ABM_REMOVE, &mut data) };
            if let Some(map) = LABELS.lock().unwrap_or_else(|e| e.into_inner()).as_mut() {
                map.remove(&(hwnd.0 as isize));
            }
        }
        // SAFETY: forwarding the unmodified message to the next window procedure.
        unsafe { DefSubclassProc(hwnd, msg, wparam, lparam) }
    }

    let _ = APP.set(window.app_handle().clone());
    let Ok(raw) = window.hwnd() else { return };
    let hwnd = HWND(raw.0);
    LABELS
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .get_or_insert_with(HashMap::new)
        .insert(hwnd.0 as isize, window.label().to_owned());
    // SAFETY: called on the main thread that owns this live window; the callback only reads
    // statics and forwards every message unchanged.
    let installed = unsafe { SetWindowSubclass(hwnd, Some(subclass), 1, 0) };
    if !installed.as_bool() {
        tracing::warn!("could not watch for display changes; the island may drift until restart");
        return;
    }
    let mut data = appbar(hwnd);
    // SAFETY: registers this window for app-bar notifications; no screen space is claimed.
    if unsafe { SHAppBarMessage(ABM_NEW, &mut data) } == 0 {
        tracing::warn!("could not watch for fullscreen apps");
    }
    // Notifications only report changes; ask once whether something is fullscreen already,
    // and whether it's on this island's screen.
    // SAFETY: plain queries with no arguments or with window handles owned by the system.
    let busy = unsafe { SHQueryUserNotificationState() }.is_ok_and(|state| {
        matches!(
            state,
            QUNS_BUSY | QUNS_RUNNING_D3D_FULL_SCREEN | QUNS_PRESENTATION_MODE
        )
    });
    let same_screen = unsafe {
        MonitorFromWindow(GetForegroundWindow(), MONITOR_DEFAULTTONEAREST)
            == MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST)
    };
    set_fullscreen(window.app_handle(), window.label(), busy && same_screen);
}
