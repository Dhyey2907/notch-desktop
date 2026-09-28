//! Desktop shell: island window placement, tray, global shortcut, and window lifecycle
//! (architecture.md §3–4). The frontend decides *what size* the island needs; this
//! module decides *where* that lands on the display.

mod placement;

use std::sync::Mutex;

use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Manager, WebviewUrl, WebviewWindow, WebviewWindowBuilder, WindowEvent,
};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};

use crate::{error::AppResult, events::AppEvent, state::AppState};
use placement::{top_center_frame, WorkArea};

pub const MAIN_WINDOW: &str = "main";
pub const SETTINGS_WINDOW: &str = "settings";
/// Passed by the Windows startup entry so a sign-in launch can be told apart from a manual one.
pub const AUTOSTART_ARG: &str = "--autostart";

/// Open the settings window, or focus it if it is already open. It is an ordinary
/// decorated window, created on demand and destroyed on close (design.md §21).
pub fn open_settings_window(app: &AppHandle) -> AppResult<()> {
    if let Some(window) = app.get_webview_window(SETTINGS_WINDOW) {
        window.unminimize()?;
        window.show()?;
        window.set_focus()?;
        return Ok(());
    }
    WebviewWindowBuilder::new(
        app,
        SETTINGS_WINDOW,
        WebviewUrl::App("index.html#settings".into()),
    )
    .title("Notch Desktop Settings")
    .inner_size(760.0, 540.0)
    .min_inner_size(640.0, 440.0)
    .center()
    .build()?;
    Ok(())
}

/// Logical window size requested by the frontend, reused when the display changes.
pub struct ShellState {
    window_size: Mutex<(f64, f64)>,
}

impl ShellState {
    pub fn new(initial: (f64, f64)) -> Self {
        Self {
            window_size: Mutex::new(initial),
        }
    }

    pub fn window_size(&self) -> (f64, f64) {
        *self.window_size.lock().unwrap_or_else(|e| e.into_inner())
    }

    pub fn set_window_size(&self, size: (f64, f64)) {
        *self.window_size.lock().unwrap_or_else(|e| e.into_inner()) = size;
    }
}

/// Size and position the window top-center on the primary monitor (MVP: design.md §20).
pub fn place(window: &WebviewWindow, (width, height): (f64, f64)) -> AppResult<()> {
    let Some(monitor) = window.primary_monitor()?.or(window.current_monitor()?) else {
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

fn main_window(app: &AppHandle) -> Option<WebviewWindow> {
    app.get_webview_window(MAIN_WINDOW)
}

fn toggle_visibility(app: &AppHandle) {
    let Some(window) = main_window(app) else { return };
    let result = if window.is_visible().unwrap_or(false) {
        window.hide().map_err(Into::into)
    } else {
        show_island(&window)
    };
    if let Err(error) = result {
        tracing::warn!(%error, "failed to toggle island visibility");
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
                if let Err(error) = open_settings_window(app) {
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
            if let Some(window) = main_window(app) {
                let _ = show_island(&window);
                let _ = window.set_focus();
            }
            app.state::<AppState>()
                .bus
                .emit("shell", AppEvent::ShellToggleRequested);
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

    let Some(window) = main_window(app) else {
        return Ok(());
    };
    let handle = app.clone();
    window.on_window_event(move |event| match event {
        // No decorations, but Alt+F4 still arrives: hide to tray instead of quitting.
        WindowEvent::CloseRequested { api, .. } => {
            api.prevent_close();
            if let Some(window) = main_window(&handle) {
                let _ = window.hide();
            }
        }
        WindowEvent::ScaleFactorChanged { .. } => {
            if let Some(window) = main_window(&handle) {
                let size = handle.state::<ShellState>().window_size();
                if let Err(error) = place(&window, size) {
                    tracing::warn!(%error, "failed to re-place island after scale change");
                }
            }
        }
        _ => {}
    });

    place(&window, app.state::<ShellState>().window_size())?;
    show_island(&window)?;
    Ok(())
}
