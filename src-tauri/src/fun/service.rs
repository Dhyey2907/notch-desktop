//! PC lights following the notch (through OpenRGB) and the pet's walk along the taskbar.

use std::{
    sync::{
        atomic::{AtomicBool, AtomicU32, Ordering},
        Arc, Mutex,
    },
    time::{Duration, Instant},
};

use tauri::{AppHandle, Manager, PhysicalPosition, PhysicalSize, WebviewUrl, WebviewWindowBuilder};

use super::{openrgb::Client, parse_hex, shade, RgbStatus};
use crate::{events::AppEvent, settings::FunSettings, state::AppState};

const DEFAULT_PORT: u16 = 6742;
/// About 15 updates a second while music plays.
const RGB_TICK: Duration = Duration::from_millis(66);
const RECONNECT_EVERY: Duration = Duration::from_secs(10);
/// White-blue, the notch's own accent when none is chosen.
const DEFAULT_ACCENT: (u8, u8, u8) = (138, 180, 255);
const RESEND_EVERY: Duration = Duration::from_secs(2);
const BEAT_FLASH: Duration = Duration::from_millis(120);

pub const PET_WINDOW: &str = "petwalk";
/// Away this long and the pet goes for a walk.
const PET_AFTER_MS: u64 = 2 * 60_000;
const PET_TICK: Duration = Duration::from_secs(3);
/// While the pet is out, how often to look for a click.
const PET_WATCH: Duration = Duration::from_millis(40);
/// Steady typing for this long also sends it away.
const PET_BUSY_FOR: Duration = Duration::from_secs(15);
/// The jump and the run off the screen (matches the pet window's animation).
const PET_LEAVE: Duration = Duration::from_millis(1700);
/// Tells the pet window to jump and run off.
pub const PET_LEAVE_EVENT: &str = "pet-leave";
/// Tells the pet window where you clicked inside it (it knows where the pet is, and whether
/// that was a click on the pet: a treat).
pub const PET_CLICK_EVENT: &str = "pet-click";
/// Tells the pet window where the mouse is while it's over the pet's strip (and once when it
/// leaves), so the pet can be petted, chase the cursor, or be picked up.
pub const PET_POINTER_EVENT: &str = "pet-pointer";
/// Tells every window whether the pet is out walking: there is one pet, so while it walks it
/// isn't in the notch.
pub const PET_WALKING_EVENT: &str = "pet-walking";
/// Logical pixels; scaled for the screen. Room for the pet and its speech cloud above it.
const PET_HEIGHT: f64 = 130.0;
/// How often to look where the playing app's windows are (which screen).
const MEDIA_PLACE_EVERY: Duration = Duration::from_secs(3);
/// While waiting, how often to check whether it was just called out.
const PET_NAP: Duration = Duration::from_millis(100);

#[derive(Default)]
struct Inner {
    /// The album art color while music plays (set by the island).
    music: Option<String>,
    beat_at: Option<Instant>,
    status: Option<RgbStatus>,
}

#[derive(Clone)]
pub struct FunService {
    app: AppHandle,
    inner: Arc<Mutex<Inner>>,
    /// Music loudness 0..1 as f32 bits.
    level: Arc<AtomicU32>,
    /// The pet is out walking the taskbar.
    walking: Arc<AtomicBool>,
    /// "Send back" was pressed while it walks.
    send_back: Arc<AtomicBool>,
    /// It was just called out: don't finish the current wait.
    wake: Arc<AtomicBool>,
    /// The pointer is on the pet: its window takes the mouse (to pet, carry or throw it)
    /// instead of letting clicks through to the taskbar.
    captured: Arc<AtomicBool>,
}

impl FunService {
    pub fn new(app: AppHandle) -> Self {
        Self {
            app,
            inner: Arc::default(),
            level: Arc::default(),
            walking: Arc::default(),
            send_back: Arc::default(),
            wake: Arc::default(),
            captured: Arc::default(),
        }
    }

    fn settings(&self) -> (FunSettings, String) {
        let s = self.app.state::<AppState>();
        let s = s.settings.lock().unwrap_or_else(|e| e.into_inner());
        (s.fun.clone(), s.appearance.accent.clone())
    }

    /// The pet is out walking the taskbar right now.
    pub fn pet_walking(&self) -> bool {
        self.walking.load(Ordering::Relaxed)
    }

    /// "Send back": the walking pet jumps and runs off (the pet loop sees this on its next look).
    pub fn send_pet_back(&self) {
        if self.pet_walking() {
            self.send_back.store(true, Ordering::Relaxed);
        }
    }

    fn set_walking(&self, walking: bool) {
        use tauri::Emitter;
        self.send_back.store(false, Ordering::Relaxed);
        if self.walking.swap(walking, Ordering::Relaxed) != walking {
            let _ = self.app.emit(PET_WALKING_EVENT, walking);
        }
    }

    /// Called out just now: the pet loop looks again right away instead of finishing its wait.
    pub fn wake_pet(&self) {
        self.wake.store(true, Ordering::Relaxed);
    }

    /// Wait up to `total`, less if the pet is called out meanwhile.
    fn nap(&self, total: Duration) {
        let until = Instant::now() + total;
        while Instant::now() < until && !self.wake.swap(false, Ordering::Relaxed) {
            std::thread::sleep(PET_NAP.min(until.saturating_duration_since(Instant::now())));
        }
    }

    /// Whether a click on the screen sends the pet back, and whether it was called out.
    fn pet_leave(&self) -> (bool, bool) {
        let s = self.app.state::<AppState>();
        let s = s.settings.lock().unwrap_or_else(|e| e.into_inner());
        (leaves_on_click(&s.pet.leave), s.pet.buddy_out)
    }

    /// The island tells us the music's color (`None` when nothing plays).
    pub fn set_music_color(&self, hex: Option<String>) {
        let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        inner.music = hex.filter(|h| parse_hex(h).is_some());
        if inner.music.is_none() {
            self.level.store(0, Ordering::Relaxed);
        }
    }

    pub fn status(&self) -> RgbStatus {
        self.inner
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .status
            .clone()
            .unwrap_or(RgbStatus {
                state: "off".into(),
                devices: 0,
            })
    }

    fn set_status(&self, state: &str, devices: usize) {
        self.inner.lock().unwrap_or_else(|e| e.into_inner()).status = Some(RgbStatus {
            state: state.into(),
            devices: devices as u32,
        });
    }

    pub fn start(&self) {
        // Loudness and beats, for the lights.
        let this = self.clone();
        let mut events = self.app.state::<AppState>().bus.subscribe();
        tauri::async_runtime::spawn(async move {
            loop {
                match events.recv().await {
                    Ok(envelope) => {
                        if let AppEvent::MusicPulse { level, beat } = envelope.event {
                            this.level.store(level.to_bits(), Ordering::Relaxed);
                            if beat {
                                this.inner.lock().unwrap_or_else(|e| e.into_inner()).beat_at =
                                    Some(Instant::now());
                            }
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                    Err(_) => break,
                }
            }
        });

        let this = self.clone();
        std::thread::Builder::new()
            .name("rgb".into())
            .spawn(move || this.rgb_loop())
            .ok();
        // Its own thread: window calls wait on the main thread, and must never tie up the
        // async runtime's workers while they do.
        let this = self.clone();
        std::thread::Builder::new()
            .name("pet".into())
            .spawn(move || this.pet_loop())
            .ok();
    }

    /// The color the lights should have now.
    fn target(&self, accent: &str) -> (u8, u8, u8) {
        let inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        match inner.music.as_deref().and_then(parse_hex) {
            Some(color) => {
                let level = f32::from_bits(self.level.load(Ordering::Relaxed));
                let beat = inner.beat_at.is_some_and(|at| at.elapsed() < BEAT_FLASH);
                shade(color, level, beat)
            }
            None => parse_hex(accent).unwrap_or(DEFAULT_ACCENT),
        }
    }

    fn rgb_loop(&self) {
        let mut client: Option<(Client, u16)> = None;
        let mut last_try: Option<Instant> = None;
        let mut last_color = None;
        let mut last_sent: Option<Instant> = None;
        loop {
            let (fun, accent) = self.settings();
            let port = if fun.rgb_port == 0 {
                DEFAULT_PORT
            } else {
                fun.rgb_port
            };
            if !fun.rgb {
                client = None;
                last_color = None;
                self.set_status("off", 0);
                std::thread::sleep(Duration::from_secs(1));
                continue;
            }
            if client.as_ref().is_some_and(|(_, p)| *p != port) {
                client = None;
            }
            if client.is_none() {
                if last_try.is_some_and(|t| t.elapsed() < RECONNECT_EVERY) {
                    std::thread::sleep(Duration::from_millis(500));
                    continue;
                }
                last_try = Some(Instant::now());
                match Client::connect(port) {
                    Ok(c) => {
                        tracing::info!(devices = c.devices.len(), "connected to OpenRGB");
                        self.set_status("connected", c.devices.len());
                        client = Some((c, port));
                        last_color = None;
                    }
                    Err(_) => {
                        self.set_status("notRunning", 0);
                        continue;
                    }
                }
            }
            let color = self.target(&accent);
            // Resend now and then even when nothing changed: that's how a closed OpenRGB is noticed.
            if last_color != Some(color) || last_sent.is_none_or(|t| t.elapsed() >= RESEND_EVERY) {
                if let Some((c, _)) = client.as_mut() {
                    if c.set_all(color).is_err() {
                        tracing::info!("lost the OpenRGB connection");
                        client = None;
                        self.set_status("notRunning", 0);
                        continue;
                    }
                }
                last_color = Some(color);
                last_sent = Some(Instant::now());
            }
            std::thread::sleep(RGB_TICK);
        }
    }

    /// The pet's walk along the taskbar.
    ///
    /// It walks while you're away: then it goes back when you click the screen (or type for a
    /// while), or only when you send it back, whichever you chose. Called out by you, it keeps
    /// walking until you send it back; clicks never send it away.
    ///
    /// Only its own screen matters. A fullscreen app, a playing video, a click or your typing
    /// on another screen don't affect it.
    fn pet_loop(&self) {
        use super::buddy::{self, Place};
        let mut out = false;
        let mut busy_since: Option<Instant> = None;
        let mut media_place = Place::Unknown;
        let mut media_checked: Option<Instant> = None;
        let mut was_down = false;
        let mut last_pointer: Option<(i64, i64, bool)> = None;
        loop {
            let (fun, _) = self.settings();
            let (on_click, called_out) = self.pet_leave();
            let (down, tapped) = mouse_buttons();
            // A new press, not a button still held from before.
            let pressed = (down || tapped) && !was_down;
            was_down = down;
            #[cfg(windows)]
            let (idle, locked) = (
                crate::system::screentime::probe::idle_ms(),
                crate::system::screentime::probe::locked(),
            );
            #[cfg(not(windows))]
            let (idle, locked) = (0, false);
            let playing = self
                .app
                .state::<crate::system::service::SystemService>()
                .media_playing();
            // Something playing only on another screen isn't the pet's business.
            if playing && media_checked.is_none_or(|at| at.elapsed() >= MEDIA_PLACE_EVERY) {
                media_place = buddy::media_place(&self.app);
                media_checked = Some(Instant::now());
            }
            // The pet walks on the screen the notch is on; only a fullscreen app there hides it.
            let fullscreen_here = crate::shell::is_fullscreen(crate::shell::MAIN_WINDOW);
            let wanted = wants_walk(Walk {
                called_out,
                walk_when_away: fun.pet_walk,
                locked,
                playing_here: playing && media_place != Place::Elsewhere,
                fullscreen_here,
            });
            if !out {
                // Demo mode (recording the website's videos) doesn't wait two minutes.
                let after = if crate::demo() { 8_000 } else { PET_AFTER_MS };
                if wanted && (called_out || idle >= after) {
                    out = self.show_pet();
                    self.set_walking(out);
                    busy_since = None;
                }
                if out {
                    std::thread::sleep(PET_WATCH);
                } else {
                    self.nap(PET_TICK);
                }
                continue;
            }
            let asked = self.send_back.swap(false, Ordering::Relaxed);
            if !wanted && !asked {
                self.hide_pet();
                self.set_walking(false);
                out = false;
                continue;
            }
            // While its window has the mouse, it hears clicks itself.
            let captured = self.captured.load(Ordering::Relaxed);
            // You called it out and clicked: if that was on the pet, it gets a treat.
            if called_out && pressed && !captured {
                self.pet_clicked();
            }
            self.pet_pointer(down, &mut last_pointer);
            // Back at work means on the pet's screen: clicking or typing on another one doesn't
            // count. Playing with the pet itself doesn't either.
            let clicked_here = (down || tapped) && !captured && buddy::cursor_on_pet_screen(&self.app);
            busy_since = if idle < 1500 && buddy::front_on_pet_screen(&self.app) {
                busy_since.or(Some(Instant::now()))
            } else {
                None
            };
            let typing_here = busy_since.is_some_and(|t| t.elapsed() >= PET_BUSY_FOR);
            if leaves_now(Leave {
                asked,
                called_out,
                on_click,
                back_at_work: clicked_here || typing_here,
            }) {
                use tauri::Emitter;
                let _ = self.app.emit_to(PET_WINDOW, PET_LEAVE_EVENT, ());
                std::thread::sleep(PET_LEAVE);
                self.hide_pet();
                self.set_walking(false);
                out = false;
                // Back in the notch: it's no longer called out.
                if let Err(error) = buddy::hide(&self.app) {
                    tracing::warn!(kind = error.kind(), "could not save that the pet went back");
                }
                continue;
            }
            std::thread::sleep(PET_WATCH);
        }
    }

    /// Where the mouse is over the pet's strip, in the window's own pixels, sent only when it
    /// moved or a button changed; one `inside: false` when it leaves the strip.
    fn pet_pointer(&self, down: bool, last: &mut Option<(i64, i64, bool)>) {
        use tauri::Emitter;
        let Some(w) = self.app.get_webview_window(PET_WINDOW) else {
            return;
        };
        let (Ok(pos), Ok(size), Ok(scale), Some(cursor)) = (
            w.outer_position(),
            w.outer_size(),
            w.scale_factor(),
            super::buddy::cursor(),
        ) else {
            return;
        };
        match point_in_window(cursor, (pos.x, pos.y), (size.width, size.height), scale) {
            Some((x, y)) => {
                let now = (x.round() as i64, y.round() as i64, down);
                if *last != Some(now) {
                    *last = Some(now);
                    let _ = self.app.emit_to(
                        PET_WINDOW,
                        PET_POINTER_EVENT,
                        serde_json::json!({ "x": x, "y": y, "down": down, "inside": true }),
                    );
                }
            }
            None => {
                if last.take().is_some() {
                    let _ = self.app.emit_to(
                        PET_WINDOW,
                        PET_POINTER_EVENT,
                        serde_json::json!({ "x": -1, "y": -1, "down": false, "inside": false }),
                    );
                }
            }
        }
    }

    /// The pet's window takes the mouse while the pointer is on the pet (to pet, carry or
    /// throw it), and lets it through again when it leaves.
    pub fn capture_pet(&self, on: bool) {
        if let Some(w) = self.app.get_webview_window(PET_WINDOW) {
            if set_click_through(&w, !on) {
                self.captured.store(on, Ordering::Relaxed);
            }
        }
    }

    /// A click while the pet is out: if it landed inside the pet's strip, tell the pet's window
    /// where (it knows where the pet is right now). The window lets clicks through, so this is
    /// how it hears about them.
    fn pet_clicked(&self) {
        use tauri::Emitter;
        let Some(w) = self.app.get_webview_window(PET_WINDOW) else {
            return;
        };
        let (Ok(pos), Ok(size), Ok(scale), Some(cursor)) = (
            w.outer_position(),
            w.outer_size(),
            w.scale_factor(),
            super::buddy::cursor(),
        ) else {
            return;
        };
        if let Some((x, y)) = point_in_window(cursor, (pos.x, pos.y), (size.width, size.height), scale) {
            let _ = self
                .app
                .emit_to(PET_WINDOW, PET_CLICK_EVENT, serde_json::json!({ "x": x, "y": y }));
        }
    }

    /// Bring the pet out (opening its window the first time). Returns whether it's showing.
    fn show_pet(&self) -> bool {
        match self.app.get_webview_window(PET_WINDOW) {
            Some(w) => {
                self.place_pet(&w);
                w.show().is_ok()
            }
            None => match self.open_pet() {
                Ok(()) => true,
                Err(e) => {
                    tracing::warn!(error = %e, "could not open the walking pet");
                    false
                }
            },
        }
    }

    fn hide_pet(&self) {
        if let Some(w) = self.app.get_webview_window(PET_WINDOW) {
            let _ = w.hide();
            set_click_through(&w, true);
        }
        self.captured.store(false, Ordering::Relaxed);
    }

    fn open_pet(&self) -> tauri::Result<()> {
        let w = WebviewWindowBuilder::new(&self.app, PET_WINDOW, WebviewUrl::App("index.html#pet".into()))
            .title("Overline pet")
            .decorations(false)
            .transparent(true)
            .always_on_top(true)
            .skip_taskbar(true)
            .resizable(false)
            .shadow(false)
            .focused(false)
            .focusable(false)
            .visible(false)
            .build()?;
        w.set_ignore_cursor_events(true)?;
        self.place_pet(&w);
        w.show()
    }

    /// Standing on the taskbar of the screen the notch is on.
    fn place_pet(&self, w: &tauri::WebviewWindow) {
        let notch_screen = self
            .app
            .get_webview_window(crate::shell::MAIN_WINDOW)
            .and_then(|main| main.current_monitor().ok().flatten());
        let Some(monitor) = notch_screen.or_else(|| w.primary_monitor().ok().flatten()) else {
            return;
        };
        let area = monitor.work_area();
        let (y, height) = pet_strip(
            (area.position.y, area.size.height),
            (PET_HEIGHT * monitor.scale_factor()).round() as u32,
        );
        let _ = w.set_size(PhysicalSize::new(area.size.width, height));
        let _ = w.set_position(PhysicalPosition::new(area.position.x, y));
    }
}

/// Let the mouse through the pet's window, or take it. Only flips `WS_EX_TRANSPARENT`: Tauri's
/// `set_ignore_cursor_events` rewrites the whole window style and drops `WS_EX_LAYERED` too,
/// which on a transparent window leaves ghost trails and can draw a classic window frame (that
/// also lifts the pet off the taskbar). The window keeps `WS_EX_LAYERED` from when it opened.
#[cfg(windows)]
fn set_click_through(w: &tauri::WebviewWindow, through: bool) -> bool {
    use windows::Win32::UI::WindowsAndMessaging::{
        GetWindowLongPtrW, SetWindowLongPtrW, GWL_EXSTYLE, WS_EX_LAYERED, WS_EX_TRANSPARENT,
    };
    let Ok(raw) = w.hwnd() else {
        return false;
    };
    // Tauri may link a different `windows` version; both HWNDs wrap the same raw handle.
    let hwnd = windows::Win32::Foundation::HWND(raw.0);
    // SAFETY: `hwnd` is this process's live pet window; only extended-style bits change.
    unsafe {
        let style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
        let layered = style | WS_EX_LAYERED.0 as isize;
        let next = if through {
            layered | WS_EX_TRANSPARENT.0 as isize
        } else {
            layered & !(WS_EX_TRANSPARENT.0 as isize)
        };
        if next != style {
            SetWindowLongPtrW(hwnd, GWL_EXSTYLE, next);
        }
    }
    true
}

#[cfg(not(windows))]
fn set_click_through(w: &tauri::WebviewWindow, through: bool) -> bool {
    w.set_ignore_cursor_events(through).is_ok()
}

/// Where the pet walks: a strip whose bottom is the taskbar's top edge (the bottom of the work
/// area), so the pet stands on the taskbar. `(top, height)` in physical pixels.
fn pet_strip(work: (i32, u32), height: u32) -> (i32, u32) {
    (work.0 + work.1 as i32 - height as i32, height)
}

/// A screen point (physical pixels) as a point inside a window, in the window's own logical
/// pixels; `None` when it's outside the window.
fn point_in_window(
    point: (i32, i32),
    origin: (i32, i32),
    size: (u32, u32),
    scale: f64,
) -> Option<(f64, f64)> {
    let (dx, dy) = (point.0 - origin.0, point.1 - origin.1);
    if dx < 0 || dy < 0 || dx >= size.0 as i32 || dy >= size.1 as i32 || scale <= 0.0 {
        return None;
    }
    Some((f64::from(dx) / scale, f64::from(dy) / scale))
}

/// What decides whether the pet may be out walking right now.
#[derive(Clone, Copy)]
struct Walk {
    /// You called it out of the notch.
    called_out: bool,
    /// "Walks the taskbar" (while you're away) is on.
    walk_when_away: bool,
    locked: bool,
    /// Music or a video is playing, and not only on another screen.
    playing_here: bool,
    /// A fullscreen app covers the pet's own screen.
    fullscreen_here: bool,
}

/// Never behind the lock screen, never over a fullscreen app on its own screen. Called out, it
/// walks whatever else is going on; otherwise it only walks while you're away, and something
/// playing means you're watching or listening, not away.
fn wants_walk(w: Walk) -> bool {
    if w.locked || w.fullscreen_here {
        return false;
    }
    w.called_out || (w.walk_when_away && !w.playing_here)
}

/// What decides whether the pet goes back into the notch right now.
#[derive(Clone, Copy)]
struct Leave {
    /// "Send back" was pressed.
    asked: bool,
    /// You called it out yourself (rather than it walking because you were away).
    called_out: bool,
    /// The "Goes back" choice is "When I click the screen".
    on_click: bool,
    /// You clicked, or have been typing for a while, on the pet's own screen.
    back_at_work: bool,
}

/// Sent back, it always goes. A pet you called out goes only then. One that walked off while
/// you were away also goes when you're back at work, if that's what you chose.
fn leaves_now(l: Leave) -> bool {
    l.asked || (!l.called_out && l.on_click && l.back_at_work)
}

/// "manual" keeps the pet out until you send it back; anything else (the default, "click")
/// lets a click on the screen send it back.
fn leaves_on_click(leave: &str) -> bool {
    leave != "manual"
}

/// The mouse buttons: `(one is down now, one was pressed since the last look)`.
fn mouse_buttons() -> (bool, bool) {
    #[cfg(windows)]
    {
        use windows::Win32::UI::Input::KeyboardAndMouse::{
            GetAsyncKeyState, VK_LBUTTON, VK_MBUTTON, VK_RBUTTON,
        };
        let mut state = (false, false);
        for key in [VK_LBUTTON, VK_RBUTTON, VK_MBUTTON] {
            // SAFETY: reads the asynchronous key state; no pointers involved.
            let bits = unsafe { GetAsyncKeyState(i32::from(key.0)) } as u16;
            state.0 |= bits & 0x8000 != 0;
            state.1 |= bits & 0x0001 != 0;
        }
        state
    }
    #[cfg(not(windows))]
    (false, false)
}

#[cfg(test)]
mod pet_tests {
    use super::{leaves_now, leaves_on_click, pet_strip, point_in_window, wants_walk, Leave, Walk};

    #[test]
    fn a_click_is_placed_inside_the_pet_window_in_its_own_pixels() {
        // The strip: starts at (0, 890) on a 150% screen, 2880 x 195 physical pixels.
        let strip = ((0, 890), (2880, 195));
        assert_eq!(
            point_in_window((300, 1040), strip.0, strip.1, 1.5),
            Some((200.0, 100.0))
        );
        // Above the strip, or on another screen: not a click in the pet's window.
        assert_eq!(point_in_window((300, 800), strip.0, strip.1, 1.5), None);
        assert_eq!(point_in_window((-50, 1000), strip.0, strip.1, 1.5), None);
        assert_eq!(point_in_window((2880, 1000), strip.0, strip.1, 1.5), None);
    }

    #[test]
    fn a_pet_you_called_out_only_goes_when_you_send_it_back() {
        let called = Leave {
            asked: false,
            called_out: true,
            on_click: true,
            back_at_work: true,
        };
        assert!(!leaves_now(called));
        assert!(leaves_now(Leave {
            asked: true,
            ..called
        }));
    }

    #[test]
    fn the_away_walk_ends_on_a_click_only_if_you_chose_that() {
        let away = Leave {
            asked: false,
            called_out: false,
            on_click: true,
            back_at_work: true,
        };
        assert!(leaves_now(away));
        assert!(!leaves_now(Leave {
            back_at_work: false,
            ..away
        }));
        // "When I send it back": it keeps walking after you're back.
        assert!(!leaves_now(Leave {
            on_click: false,
            ..away
        }));
        assert!(leaves_now(Leave {
            on_click: false,
            asked: true,
            ..away
        }));
    }

    const AWAY: Walk = Walk {
        called_out: false,
        walk_when_away: true,
        locked: false,
        playing_here: false,
        fullscreen_here: false,
    };

    #[test]
    fn called_out_it_walks_even_while_something_plays_or_the_away_walk_is_off() {
        let called = Walk {
            called_out: true,
            walk_when_away: false,
            playing_here: true,
            ..AWAY
        };
        assert!(wants_walk(called));
    }

    #[test]
    fn the_away_walk_needs_its_switch_and_nothing_playing() {
        assert!(wants_walk(AWAY));
        assert!(!wants_walk(Walk {
            walk_when_away: false,
            ..AWAY
        }));
        assert!(!wants_walk(Walk {
            playing_here: true,
            ..AWAY
        }));
    }

    #[test]
    fn only_its_own_screen_going_fullscreen_or_the_lock_screen_hides_it() {
        // `fullscreen_here` is the pet's own screen; another screen's fullscreen app never
        // reaches this check.
        assert!(!wants_walk(Walk {
            fullscreen_here: true,
            ..AWAY
        }));
        assert!(!wants_walk(Walk {
            called_out: true,
            fullscreen_here: true,
            ..AWAY
        }));
        assert!(!wants_walk(Walk {
            called_out: true,
            locked: true,
            ..AWAY
        }));
    }

    #[test]
    fn only_manual_keeps_the_pet_out() {
        assert!(leaves_on_click("click"));
        assert!(!leaves_on_click("manual"));
        // An empty or unknown value (an older settings file) behaves like the default.
        assert!(leaves_on_click(""));
        assert!(leaves_on_click("something-else"));
    }

    #[test]
    fn stands_on_the_taskbar_edge() {
        // 1080p screen with a 48 px taskbar: the strip ends exactly where the taskbar starts.
        assert_eq!(pet_strip((0, 1032), 80), (952, 80));
        // A second screen below the first.
        assert_eq!(pet_strip((1080, 1368), 120), (2328, 120));
    }
}
