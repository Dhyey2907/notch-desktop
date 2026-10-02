//! Game mode: when a fullscreen app is one of your games, read FPS (PresentMon), GPU (NVML),
//! CPU, RAM and ping every second and send the HUD to the notch until the game leaves
//! fullscreen. Unknown fullscreen apps are asked about once.

use std::{
    collections::HashSet,
    io::{BufRead, BufReader},
    path::PathBuf,
    process::{Child, Command, Stdio},
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, Mutex,
    },
    time::Duration,
};

use serde::Serialize;
use tauri::{AppHandle, Manager};
use ts_rs::TS;

use super::{Columns, FpsMeter, GameHud, PRESENTMON_FILE, PRESENTMON_SHA256, PRESENTMON_URL};
use crate::{
    error::{AppError, AppResult},
    events::AppEvent,
    state::AppState,
};

/// Only ask about an app that stays fullscreen this long: screenshot overlays and other
/// brief fullscreen moments shouldn't trigger the question.
const ASK_AFTER: Duration = Duration::from_secs(10);
const TICK: Duration = Duration::from_secs(1);
/// Ping every few ticks (a round trip each second would be noisy and pointless).
const PING_EVERY_TICKS: u64 = 3;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct GameStatus {
    /// PresentMon is downloaded and verified, so FPS can be shown.
    pub fps_ready: bool,
    /// The game being played now, if any.
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub playing: Option<String>,
}

struct Active {
    session: u64,
    app: String,
    pid: u32,
    presentmon: Option<Child>,
}

impl Drop for Active {
    fn drop(&mut self) {
        if let Some(child) = self.presentmon.as_mut() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

#[derive(Clone)]
pub struct GameService {
    app: AppHandle,
    tools: PathBuf,
    active: Arc<Mutex<Option<Active>>>,
    meter: Arc<Mutex<FpsMeter>>,
    session: Arc<AtomicU64>,
    /// Apps already asked about since Notch started (answered or not): never asked twice.
    asked: Arc<Mutex<HashSet<String>>>,
}

fn front_app() -> Option<(u32, String)> {
    #[cfg(windows)]
    {
        use windows::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowThreadProcessId};
        let mut pid = 0;
        // SAFETY: plain queries on the foreground window.
        unsafe { GetWindowThreadProcessId(GetForegroundWindow(), Some(&mut pid)) };
        let exe = crate::clipboard::win::exe_of(pid)?;
        Some((pid, exe))
    }
    #[cfg(not(windows))]
    None
}

impl GameService {
    pub fn new(app: AppHandle, tools: PathBuf) -> Self {
        Self {
            app,
            tools,
            active: Arc::default(),
            meter: Arc::default(),
            session: Arc::default(),
            asked: Arc::default(),
        }
    }

    fn presentmon_path(&self) -> PathBuf {
        self.tools.join(PRESENTMON_FILE)
    }

    /// The tool is there and is exactly the pinned build.
    fn presentmon_ready(&self) -> bool {
        std::fs::read(self.presentmon_path())
            .is_ok_and(|bytes| super::sha256_hex(&bytes) == PRESENTMON_SHA256)
    }

    pub fn status(&self) -> GameStatus {
        GameStatus {
            fps_ready: self.presentmon_ready(),
            playing: self
                .active
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .as_ref()
                .map(|a| a.app.clone()),
        }
    }

    fn settings(&self) -> crate::settings::GameSettings {
        self.app
            .state::<AppState>()
            .settings
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .game
            .clone()
    }

    pub fn start(&self) {
        let this = self.clone();
        let mut events = self.app.state::<AppState>().bus.subscribe();
        tauri::async_runtime::spawn(async move {
            loop {
                match events.recv().await {
                    Ok(envelope) => {
                        if let AppEvent::FullscreenChanged { label, active } = envelope.event {
                            let this = this.clone();
                            // Process lookups and PresentMon start off the async runtime.
                            tauri::async_runtime::spawn_blocking(move || this.fullscreen(&label, active));
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                    Err(_) => break,
                }
            }
        });
    }

    fn fullscreen(&self, label: &str, active: bool) {
        let settings = self.settings();
        if !active || settings.disabled {
            // Fullscreen ended on some screen: if our game is no longer in front, stop.
            let still = front_app().is_some_and(|(pid, _)| {
                self.active
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .as_ref()
                    .is_some_and(|a| a.pid == pid)
            });
            if !still || settings.disabled {
                self.end();
            }
            return;
        }
        let Some((pid, exe_path)) = front_app() else {
            return;
        };
        let exe = crate::clipboard::exe_key(&exe_path);
        let app = crate::clipboard::app_label(&exe_path);
        if self
            .active
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .as_ref()
            .is_some_and(|a| a.pid == pid)
        {
            return;
        }
        // Another fullscreen app took over from the game.
        self.end();
        if settings.games.contains(&exe) {
            self.begin(pid, app);
        } else if !super::never_a_game(&exe) && !settings.not_games.contains(&exe) {
            self.ask_later(label.to_string(), pid, exe, app);
        }
    }

    /// Ask "is this a game?" once the app has stayed fullscreen for a while, at most once
    /// per app while Notch runs.
    fn ask_later(&self, label: String, pid: u32, exe: String, app: String) {
        if self
            .asked
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .contains(&exe)
        {
            return;
        }
        let this = self.clone();
        std::thread::spawn(move || {
            std::thread::sleep(ASK_AFTER);
            let settings = this.settings();
            let still = crate::shell::is_fullscreen(&label) && front_app().is_some_and(|(p, _)| p == pid);
            if !still
                || settings.disabled
                || settings.games.contains(&exe)
                || settings.not_games.contains(&exe)
                || !this
                    .asked
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .insert(exe.clone())
            {
                return;
            }
            this.app
                .state::<AppState>()
                .bus
                .emit("game", AppEvent::GameAsk { exe, app });
        });
    }

    /// Remember whether an app is a game; a "yes" starts game mode if it's in front now.
    pub fn mark(&self, exe: &str, is_game: bool) -> AppResult<()> {
        let exe = crate::clipboard::exe_key(exe);
        if exe.is_empty() || exe.len() > 100 {
            return Err(AppError::InvalidInput("invalid app".into()));
        }
        {
            let state = self.app.state::<AppState>();
            let mut settings = state.settings.lock().unwrap_or_else(|e| e.into_inner()).clone();
            settings.game.games.retain(|g| *g != exe);
            settings.game.not_games.retain(|g| *g != exe);
            if is_game {
                settings.game.games.push(exe.clone());
            } else {
                settings.game.not_games.push(exe.clone());
            }
            crate::commands::settings::persist_and_broadcast(&state, settings)?;
        }
        if is_game {
            if let Some((pid, path)) = front_app().filter(|(_, p)| crate::clipboard::exe_key(p) == exe) {
                self.begin(pid, crate::clipboard::app_label(&path));
            }
        }
        Ok(())
    }

    fn begin(&self, pid: u32, app: String) {
        let session = self.session.fetch_add(1, Ordering::SeqCst) + 1;
        self.meter.lock().unwrap_or_else(|e| e.into_inner()).clear();
        let presentmon = self.spawn_presentmon(pid);
        *self.active.lock().unwrap_or_else(|e| e.into_inner()) = Some(Active {
            session,
            app,
            pid,
            presentmon,
        });
        tracing::info!("game mode on");
        let this = self.clone();
        std::thread::Builder::new()
            .name("game-hud".into())
            .spawn(move || this.tick(session))
            .ok();
    }

    fn end(&self) {
        let ended = self.active.lock().unwrap_or_else(|e| e.into_inner()).take();
        if ended.is_some() {
            self.session.fetch_add(1, Ordering::SeqCst);
            drop(ended);
            tracing::info!("game mode off");
            self.app
                .state::<AppState>()
                .bus
                .emit("game", AppEvent::GameHud(None));
        }
    }

    fn spawn_presentmon(&self, pid: u32) -> Option<Child> {
        if !self.presentmon_ready() {
            return None;
        }
        let mut command = Command::new(self.presentmon_path());
        command
            .args([
                "--process_id",
                &pid.to_string(),
                "--output_stdout",
                "--no_console_stats",
                "--stop_existing_session",
                "--session_name",
                "NotchGameMode",
                "--terminate_on_proc_exit",
                "--no_track_input",
            ])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null());
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            command.creation_flags(windows::Win32::System::Threading::CREATE_NO_WINDOW.0);
        }
        let mut child = command.spawn().ok()?;
        let stdout = child.stdout.take()?;
        let meter = self.meter.clone();
        std::thread::Builder::new()
            .name("presentmon".into())
            .spawn(move || {
                let mut lines = BufReader::new(stdout).lines();
                let Some(Ok(header)) = lines.next() else { return };
                let Some(columns) = Columns::from_header(&header) else {
                    return;
                };
                for line in lines.map_while(Result::ok) {
                    if let Some((pid, ms)) = columns.frame(&line) {
                        meter.lock().unwrap_or_else(|e| e.into_inner()).add(pid, ms);
                    }
                }
            })
            .ok();
        Some(child)
    }

    fn tick(&self, session: u64) {
        let mut ticks = 0u64;
        let mut ping = None;
        while self.session.load(Ordering::SeqCst) == session {
            let app = match self.active.lock().unwrap_or_else(|e| e.into_inner()).as_ref() {
                Some(a) if a.session == session => a.app.clone(),
                _ => return,
            };
            #[cfg(windows)]
            let ((gpu_temp, gpu_load), stats) = (super::nvml::gpu(), crate::system::stats::read());
            #[cfg(windows)]
            if ticks.is_multiple_of(PING_EVERY_TICKS) {
                ping = super::ping::ping_ms();
            }
            #[cfg(not(windows))]
            let ((gpu_temp, gpu_load), stats) = (
                (None, None),
                crate::system::SystemStats {
                    cpu: None,
                    ram: None,
                    battery: None,
                    charging: false,
                },
            );
            let hud = GameHud {
                app,
                fps: self.meter.lock().unwrap_or_else(|e| e.into_inner()).fps(),
                gpu_temp,
                gpu_load,
                cpu: stats.cpu,
                ram: stats.ram,
                ping_ms: ping,
            };
            if self.session.load(Ordering::SeqCst) != session {
                return;
            }
            self.app
                .state::<AppState>()
                .bus
                .emit("game", AppEvent::GameHud(Some(hud)));
            ticks += 1;
            std::thread::sleep(TICK);
        }
    }

    /// Download PresentMon's pinned build and check its fingerprint before keeping it.
    pub async fn setup_fps(&self) -> AppResult<()> {
        if self.presentmon_ready() {
            return Ok(());
        }
        let bytes = reqwest::Client::builder()
            .timeout(Duration::from_secs(60))
            .user_agent("Notch-Desktop")
            .build()
            .map_err(|e| AppError::Platform(e.to_string()))?
            .get(PRESENTMON_URL)
            .send()
            .await
            .map_err(|_| {
                AppError::InvalidInput("Couldn't download PresentMon. Check your connection.".into())
            })?
            .error_for_status()
            .map_err(|_| AppError::InvalidInput("GitHub didn't send PresentMon".into()))?
            .bytes()
            .await
            .map_err(|_| AppError::InvalidInput("The download was cut off".into()))?;
        if super::sha256_hex(&bytes) != PRESENTMON_SHA256 {
            return Err(AppError::InvalidInput(
                "The downloaded PresentMon didn't match the expected file, so it wasn't kept".into(),
            ));
        }
        std::fs::create_dir_all(&self.tools)?;
        let partial = self.tools.join(format!("{PRESENTMON_FILE}.part"));
        std::fs::write(&partial, &bytes)?;
        std::fs::rename(&partial, self.presentmon_path())?;
        tracing::info!("presentmon installed");
        Ok(())
    }
}
