//! One island per chosen screen (Settings → General → "Show the notch on").
//!
//! The island on the lead screen is always the `main` window; each additional screen gets an
//! `island-<device>` window with the same configuration. `sync` reconciles the open windows
//! with the setting and the connected screens, and runs at startup, when the setting changes,
//! and when the display layout changes (screens plugged in or out).

use std::{collections::HashMap, sync::Mutex};

use serde::Serialize;
use tauri::{AppHandle, Manager, Monitor, WebviewUrl, WebviewWindow, WebviewWindowBuilder};
use ts_rs::TS;

use super::MAIN_WINDOW;
use crate::{
    error::AppResult,
    settings::{DisplayMode, DisplaySettings},
    state::AppState,
};

const EXTRA_PREFIX: &str = "island-";

/// A connected screen, as offered in Settings.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ScreenInfo {
    /// Windows device name, e.g. `\\.\DISPLAY2`; what the setting stores.
    pub name: String,
    /// "Screen 2 · 1920×1080".
    pub label: String,
    pub primary: bool,
}

/// Screens that should show an island, lead screen first.
pub fn choose_screens(display: &DisplaySettings, screens: &[ScreenInfo]) -> Vec<String> {
    let Some(primary) = screens.iter().find(|s| s.primary).or(screens.first()) else {
        return Vec::new();
    };
    match display.mode {
        DisplayMode::Primary => vec![primary.name.clone()],
        DisplayMode::One => {
            let chosen = display
                .monitor
                .as_ref()
                .filter(|name| screens.iter().any(|s| &s.name == *name));
            vec![chosen.unwrap_or(&primary.name).clone()]
        }
        DisplayMode::All => std::iter::once(primary.name.clone())
            .chain(
                screens
                    .iter()
                    .filter(|s| s.name != primary.name)
                    .map(|s| s.name.clone()),
            )
            .collect(),
    }
}

/// Window label for each chosen screen: the lead screen is `main`, others `island-<device>`.
pub fn assign_labels(screens: &[String]) -> Vec<(String, String)> {
    screens
        .iter()
        .enumerate()
        .map(|(i, screen)| {
            let label = if i == 0 {
                MAIN_WINDOW.to_owned()
            } else {
                let id: String = screen.chars().filter(|c| c.is_ascii_alphanumeric()).collect();
                format!("{EXTRA_PREFIX}{id}")
            };
            (label, screen.clone())
        })
        .collect()
}

pub fn is_island(label: &str) -> bool {
    label == MAIN_WINDOW || label.starts_with(EXTRA_PREFIX)
}

#[derive(Debug, Clone)]
struct Slot {
    size: (f64, f64),
    screen: Option<String>,
}

/// Per-island window size (requested by that island's frontend) and assigned screen.
pub struct ShellState {
    default_size: (f64, f64),
    slots: Mutex<HashMap<String, Slot>>,
}

impl ShellState {
    pub fn new(default_size: (f64, f64)) -> Self {
        Self {
            default_size,
            slots: Mutex::new(HashMap::new()),
        }
    }

    fn with<T>(&self, f: impl FnOnce(&mut HashMap<String, Slot>) -> T) -> T {
        f(&mut self.slots.lock().unwrap_or_else(|e| e.into_inner()))
    }

    fn slot<'a>(&self, slots: &'a mut HashMap<String, Slot>, label: &str) -> &'a mut Slot {
        slots.entry(label.to_owned()).or_insert(Slot {
            size: self.default_size,
            screen: None,
        })
    }

    pub fn window_size(&self, label: &str) -> (f64, f64) {
        self.with(|s| s.get(label).map_or(self.default_size, |slot| slot.size))
    }

    pub fn set_window_size(&self, label: &str, size: (f64, f64)) {
        self.with(|s| self.slot(s, label).size = size);
    }

    /// The screen this island belongs on; `None` means the primary screen.
    pub fn screen(&self, label: &str) -> Option<String> {
        self.with(|s| s.get(label).and_then(|slot| slot.screen.clone()))
    }

    fn set_screen(&self, label: &str, screen: String) {
        self.with(|s| self.slot(s, label).screen = Some(screen));
    }

    fn remove(&self, label: &str) {
        self.with(|s| s.remove(label));
    }

    /// The island whose screen is `screen`, if any.
    pub fn label_for_screen(&self, screen: &str) -> Option<String> {
        self.with(|s| {
            s.iter()
                .find(|(_, slot)| slot.screen.as_deref() == Some(screen))
                .map(|(label, _)| label.clone())
        })
    }
}

/// Connected screens, numbered like Windows does (`DISPLAY1` → "Screen 1").
pub fn list_screens(app: &AppHandle) -> Vec<ScreenInfo> {
    let primary = app
        .primary_monitor()
        .ok()
        .flatten()
        .and_then(|m| m.name().cloned());
    let mut screens: Vec<(u32, ScreenInfo)> = app
        .available_monitors()
        .unwrap_or_default()
        .iter()
        .enumerate()
        .filter_map(|(i, monitor)| {
            let name = monitor.name()?.clone();
            let number = name
                .trim_start_matches(|c: char| !c.is_ascii_digit())
                .parse()
                .unwrap_or(i as u32 + 1);
            let scale = monitor.scale_factor();
            let size = monitor.size();
            let label = format!(
                "Screen {number} · {}×{}",
                (f64::from(size.width) / scale).round(),
                (f64::from(size.height) / scale).round()
            );
            Some((
                number,
                ScreenInfo {
                    primary: primary.as_deref() == Some(name.as_str()),
                    name,
                    label,
                },
            ))
        })
        .collect();
    screens.sort_by_key(|(number, _)| *number);
    #[allow(unused_mut)]
    let mut screens: Vec<ScreenInfo> = screens.into_iter().map(|(_, s)| s).collect();
    // Dev builds only: a pretend second screen, to exercise multi-island code on one monitor.
    #[cfg(debug_assertions)]
    if std::env::var("NOTCH_FAKE_SCREEN").is_ok_and(|v| v == "1") {
        screens.push(ScreenInfo {
            name: r"\\.\FAKE9".into(),
            label: "Screen 9 · pretend (dev)".into(),
            primary: false,
        });
    }
    screens
}

/// The monitor an island belongs on (falling back to the primary, then any).
pub fn monitor_for(window: &WebviewWindow, shell: &ShellState) -> Option<Monitor> {
    let wanted = shell.screen(window.label());
    wanted
        .and_then(|name| {
            window
                .available_monitors()
                .ok()?
                .into_iter()
                .find(|m| m.name() == Some(&name))
        })
        .or_else(|| window.primary_monitor().ok().flatten())
        .or_else(|| window.current_monitor().ok().flatten())
}

/// Same configuration as the `main` window in `tauri.conf.json`.
fn build_island_window(app: &AppHandle, label: &str, size: (f64, f64)) -> AppResult<WebviewWindow> {
    Ok(
        WebviewWindowBuilder::new(app, label, WebviewUrl::App("index.html".into()))
            .title("Notch Desktop")
            .inner_size(size.0, size.1)
            .transparent(true)
            .decorations(false)
            .always_on_top(true)
            .skip_taskbar(true)
            .shadow(false)
            .resizable(false)
            .maximizable(false)
            .minimizable(false)
            .focused(false)
            .visible(false)
            .build()?,
    )
}

/// Open, move, or close islands so they match the setting and the connected screens.
/// Must not run inside a synchronous command on the main thread (window creation would
/// deadlock WebView2); call it from setup, a background thread, or an async task.
pub fn sync(app: &AppHandle) {
    let display = app
        .state::<AppState>()
        .settings
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .display
        .clone();
    let wanted = assign_labels(&choose_screens(&display, &list_screens(app)));
    let shell = app.state::<ShellState>();

    for (label, window) in app.webview_windows() {
        if label.starts_with(EXTRA_PREFIX) && !wanted.iter().any(|(l, _)| *l == label) {
            shell.remove(&label);
            if let Err(error) = window.destroy() {
                tracing::warn!(%error, "failed to close an island");
            }
        }
    }
    for (label, screen) in wanted {
        shell.set_screen(&label, screen);
        let result = match app.get_webview_window(&label) {
            Some(window) => super::place(&window, shell.window_size(&label)),
            None => build_island_window(app, &label, shell.window_size(&label))
                .and_then(|window| super::attach_island(app, &window)),
        };
        if let Err(error) = result {
            tracing::warn!(%error, "failed to set up an island");
        }
    }
}

/// Every open island window.
pub fn all(app: &AppHandle) -> Vec<WebviewWindow> {
    app.webview_windows()
        .into_iter()
        .filter(|(label, _)| is_island(label))
        .map(|(_, window)| window)
        .collect()
}

/// The island on the screen under the mouse pointer, else `main`.
pub fn under_cursor(app: &AppHandle) -> Option<WebviewWindow> {
    let target = app.cursor_position().ok().and_then(|point| {
        app.available_monitors().ok()?.into_iter().find(|m| {
            let (pos, size) = (m.position(), m.size());
            point.x >= f64::from(pos.x)
                && point.x < f64::from(pos.x) + f64::from(size.width)
                && point.y >= f64::from(pos.y)
                && point.y < f64::from(pos.y) + f64::from(size.height)
        })
    });
    let shell = app.state::<ShellState>();
    target
        .and_then(|m| m.name().cloned())
        .and_then(|name| shell.label_for_screen(&name))
        .and_then(|label| app.get_webview_window(&label))
        .or_else(|| app.get_webview_window(MAIN_WINDOW))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn screens() -> Vec<ScreenInfo> {
        ["\\\\.\\DISPLAY1", "\\\\.\\DISPLAY2", "\\\\.\\DISPLAY3"]
            .iter()
            .enumerate()
            .map(|(i, name)| ScreenInfo {
                name: (*name).to_owned(),
                label: format!("Screen {}", i + 1),
                primary: i == 1,
            })
            .collect()
    }

    fn display(mode: DisplayMode, monitor: Option<&str>) -> DisplaySettings {
        DisplaySettings {
            mode,
            monitor: monitor.map(str::to_owned),
        }
    }

    #[test]
    fn primary_mode_uses_the_windows_main_display() {
        assert_eq!(
            choose_screens(&display(DisplayMode::Primary, None), &screens()),
            ["\\\\.\\DISPLAY2"]
        );
    }

    #[test]
    fn one_mode_uses_the_chosen_screen_or_falls_back_to_primary() {
        let chosen = display(DisplayMode::One, Some("\\\\.\\DISPLAY3"));
        assert_eq!(choose_screens(&chosen, &screens()), ["\\\\.\\DISPLAY3"]);
        let unplugged = display(DisplayMode::One, Some("\\\\.\\DISPLAY9"));
        assert_eq!(choose_screens(&unplugged, &screens()), ["\\\\.\\DISPLAY2"]);
    }

    #[test]
    fn all_mode_leads_with_primary_then_the_rest() {
        assert_eq!(
            choose_screens(&display(DisplayMode::All, None), &screens()),
            ["\\\\.\\DISPLAY2", "\\\\.\\DISPLAY1", "\\\\.\\DISPLAY3"]
        );
        assert!(choose_screens(&display(DisplayMode::All, None), &[]).is_empty());
    }

    #[test]
    fn lead_screen_is_main_and_others_get_stable_labels() {
        let labels = assign_labels(&["\\\\.\\DISPLAY2".into(), "\\\\.\\DISPLAY1".into()]);
        assert_eq!(
            labels,
            [
                ("main".to_owned(), "\\\\.\\DISPLAY2".to_owned()),
                ("island-DISPLAY1".to_owned(), "\\\\.\\DISPLAY1".to_owned()),
            ]
        );
        assert!(is_island("main") && is_island("island-DISPLAY1") && !is_island("settings"));
    }

    #[test]
    fn remembers_size_and_screen_per_island() {
        let shell = ShellState::new((64.0, 20.0));
        assert_eq!(shell.window_size("main"), (64.0, 20.0));
        shell.set_window_size("main", (540.0, 350.0));
        shell.set_screen("island-DISPLAY1", "\\\\.\\DISPLAY1".into());
        assert_eq!(shell.window_size("main"), (540.0, 350.0));
        assert_eq!(shell.window_size("island-DISPLAY1"), (64.0, 20.0));
        assert_eq!(
            shell.label_for_screen("\\\\.\\DISPLAY1").as_deref(),
            Some("island-DISPLAY1")
        );
        assert_eq!(shell.screen("main"), None);
        shell.remove("island-DISPLAY1");
        assert_eq!(shell.label_for_screen("\\\\.\\DISPLAY1"), None);
    }
}
