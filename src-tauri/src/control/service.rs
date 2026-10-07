//! Keeps the control center's snapshot current and raises its alerts. Network and radio
//! changes arrive as Windows events; the battery and accessories are read every few seconds
//! (cheap driver queries), and the lock keys four times a second (two key-state reads).

use std::{
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use tauri::{AppHandle, Manager};

#[cfg(windows)]
use super::{ChargeWatch, DeviceBattery, LowWatch};
use super::{ControlSnapshot, Internet, NetSpeed, RadioKind, SystemAlert};
use crate::{error::AppResult, events::AppEvent, state::AppState};

/// Battery and accessories.
#[cfg(windows)]
const SLOW_TICK: Duration = Duration::from_secs(15);
/// Lock keys and keyboard language.
#[cfg(windows)]
const KEY_TICK: Duration = Duration::from_millis(250);
/// Windows' network events can come in bursts; read once they settle.
#[cfg(windows)]
const NETWORK_SETTLE: Duration = Duration::from_millis(800);
const DEFAULT_CHARGE_ALERT: u8 = 80;

#[derive(Default)]
struct Inner {
    snapshot: ControlSnapshot,
    /// When the internet went away (for "back after 3 min").
    offline_since: Option<Instant>,
    /// Last byte counters, for speed.
    counters: Option<(Instant, (u64, u64))>,
    /// Last per-process I/O bytes, for which apps use the network.
    app_io: Option<(Instant, std::collections::HashMap<u32, u64>)>,
    /// The speed test running now, or the last one.
    speed_test: Option<super::speedtest::SpeedTest>,
}

const SPEED_TEST_KEY: &str = "control.speedtest";

#[derive(Clone)]
pub struct ControlService {
    app: AppHandle,
    inner: Arc<Mutex<Inner>>,
}

impl ControlService {
    pub fn new(app: AppHandle) -> Self {
        Self {
            app,
            inner: Arc::default(),
        }
    }

    pub fn snapshot(&self) -> ControlSnapshot {
        self.inner
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .snapshot
            .clone()
    }

    /// Bytes per second since the last call (the open panel calls this every second).
    pub fn speed(&self) -> NetSpeed {
        #[cfg(windows)]
        {
            let (now_counts, _) = super::network::counters();
            let now = Instant::now();
            let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
            let speed = inner.counters.map_or(NetSpeed::default(), |(at, before)| {
                let ms = now.duration_since(at).as_millis() as u64;
                NetSpeed {
                    down: super::rate(before.0, now_counts.0, ms),
                    up: super::rate(before.1, now_counts.1, ms),
                }
            });
            inner.counters = Some((now, now_counts));
            speed
        }
        #[cfg(not(windows))]
        NetSpeed::default()
    }

    /// Apps with internet connections and roughly how fast each is moving data.
    pub async fn net_apps(&self) -> Vec<super::netapps::AppTraffic> {
        let samples = tauri::async_runtime::spawn_blocking(super::netapps::sample)
            .await
            .unwrap_or_default();
        let now = Instant::now();
        let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        let (before, millis) = inner.app_io.take().map_or_else(
            || (Default::default(), 0),
            |(at, io)| (io, now.duration_since(at).as_millis() as u64),
        );
        let apps = super::netapps::aggregate(&before, &samples, millis, 8);
        inner.app_io = Some((now, samples.iter().map(|s| (s.pid, s.io_bytes)).collect()));
        apps
    }

    /// The last speed test (also after a restart).
    pub fn speed_test(&self) -> Option<super::speedtest::SpeedTest> {
        if let Some(test) = self
            .inner
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .speed_test
            .clone()
        {
            return Some(test);
        }
        self.app
            .state::<AppState>()
            .db
            .with(|conn| {
                Ok(conn
                    .query_row(
                        "SELECT value FROM kv_cache WHERE key = ?1",
                        [SPEED_TEST_KEY],
                        |r| r.get::<_, String>(0),
                    )
                    .ok())
            })
            .ok()
            .flatten()
            .and_then(|json| serde_json::from_str(&json).ok())
    }

    fn set_speed_test(&self, test: super::speedtest::SpeedTest) {
        self.inner.lock().unwrap_or_else(|e| e.into_inner()).speed_test = Some(test.clone());
        self.app
            .state::<AppState>()
            .bus
            .emit("control", AppEvent::SpeedTestChanged(test));
    }

    /// Ping, download, upload — about 20 seconds. Progress arrives as `SpeedTestChanged`.
    pub async fn run_speed_test(&self) -> AppResult<super::speedtest::SpeedTest> {
        use super::speedtest::{self as st, SpeedPhase, SpeedTest};
        let running = self.speed_test().is_some_and(|t| {
            matches!(
                t.phase,
                SpeedPhase::Ping | SpeedPhase::Download | SpeedPhase::Upload
            )
        });
        if running {
            return Err(crate::error::AppError::InvalidInput(
                "A speed test is already running".into(),
            ));
        }
        let mut test = SpeedTest {
            phase: SpeedPhase::Ping,
            ping_ms: None,
            down_mbps: None,
            up_mbps: None,
            at: chrono::Utc::now(),
        };
        self.set_speed_test(test.clone());
        let http = st::client();
        test.ping_ms = st::ping(&http).await;
        if test.ping_ms.is_none() {
            test.phase = SpeedPhase::Failed;
            self.set_speed_test(test.clone());
            return Err(crate::error::AppError::InvalidInput(
                "Couldn't reach the speed test server — are you online?".into(),
            ));
        }
        test.phase = SpeedPhase::Download;
        self.set_speed_test(test.clone());
        test.down_mbps = st::download(&http).await;
        test.phase = SpeedPhase::Upload;
        self.set_speed_test(test.clone());
        test.up_mbps = st::upload(&http).await;
        test.phase = if test.down_mbps.is_some() || test.up_mbps.is_some() {
            SpeedPhase::Done
        } else {
            SpeedPhase::Failed
        };
        test.at = chrono::Utc::now();
        self.set_speed_test(test.clone());
        if test.phase == SpeedPhase::Done {
            if let Ok(json) = serde_json::to_string(&test) {
                let _ = self.app.state::<AppState>().db.with(|conn| {
                    conn.execute(
                        "INSERT INTO kv_cache (key, value, fetched_at) VALUES (?1, ?2, ?3)
                         ON CONFLICT(key) DO UPDATE SET value = excluded.value, fetched_at = excluded.fetched_at",
                        rusqlite::params![SPEED_TEST_KEY, json, test.at.to_rfc3339()],
                    )?;
                    Ok(())
                });
            }
        }
        Ok(test)
    }

    /// Read everything again now (the panel was opened).
    pub fn refresh(&self) {
        #[cfg(windows)]
        {
            self.refresh_radios();
            self.refresh_power(false);
            self.refresh_devices(false);
        }
    }

    pub fn set_radio(&self, kind: RadioKind, on: bool) -> AppResult<()> {
        #[cfg(windows)]
        {
            super::radios::set(kind, on)?;
            self.refresh_radios();
            Ok(())
        }
        #[cfg(not(windows))]
        {
            let _ = (kind, on);
            Err(crate::error::AppError::Platform("not supported".into()))
        }
    }

    fn hud(&self) -> crate::settings::HudSettings {
        self.app
            .state::<AppState>()
            .settings
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .hud
            .clone()
    }

    fn alert(&self, alert: SystemAlert) {
        self.app
            .state::<AppState>()
            .bus
            .emit("control", AppEvent::SystemAlert(alert));
    }

    /// Change part of the snapshot; tell the UI only if something actually changed.
    fn update(&self, change: impl FnOnce(&mut ControlSnapshot)) {
        let next = {
            let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
            let before = inner.snapshot.clone();
            change(&mut inner.snapshot);
            (inner.snapshot != before).then(|| inner.snapshot.clone())
        };
        if let Some(snapshot) = next {
            self.app
                .state::<AppState>()
                .bus
                .emit("control", AppEvent::ControlChanged(snapshot));
        }
    }

    pub fn start(&self) {
        #[cfg(windows)]
        {
            self.refresh_network(true);
            self.refresh_radios();
            self.refresh_power(true);
            self.refresh_devices(true);

            // A headset that connects reports its battery a little later: look now and twice more.
            let this = self.clone();
            let mut events = self.app.state::<AppState>().bus.subscribe();
            tauri::async_runtime::spawn(async move {
                loop {
                    match events.recv().await {
                        Ok(envelope) => {
                            if matches!(envelope.event, AppEvent::BluetoothChanged(_)) {
                                for wait in [1u64, 8, 30] {
                                    let this = this.clone();
                                    tauri::async_runtime::spawn(async move {
                                        tokio::time::sleep(Duration::from_secs(wait)).await;
                                        let _ = tauri::async_runtime::spawn_blocking(move || {
                                            this.refresh_devices(false)
                                        })
                                        .await;
                                    });
                                }
                            }
                        }
                        Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                        Err(_) => break,
                    }
                }
            });

            let this = self.clone();
            let pending = Arc::new(std::sync::atomic::AtomicBool::new(false));
            let token = super::network::watch(move || {
                if pending.swap(true, std::sync::atomic::Ordering::SeqCst) {
                    return;
                }
                let this = this.clone();
                let pending = pending.clone();
                std::thread::spawn(move || {
                    std::thread::sleep(NETWORK_SETTLE);
                    pending.store(false, std::sync::atomic::Ordering::SeqCst);
                    this.refresh_network(false);
                    this.refresh_radios();
                });
            });
            if token.is_none() {
                tracing::warn!("could not watch for network changes");
            }

            let this = self.clone();
            let watchers = super::radios::watch(move || {
                let this = this.clone();
                std::thread::spawn(move || this.refresh_radios());
            });
            // The radio objects must stay alive for their events to keep coming.
            std::mem::forget(watchers);

            let this = self.clone();
            std::thread::Builder::new()
                .name("control".into())
                .spawn(move || this.slow_loop())
                .ok();
            let this = self.clone();
            std::thread::Builder::new()
                .name("control-keys".into())
                .spawn(move || this.key_loop())
                .ok();
        }
    }

    #[cfg(windows)]
    fn refresh_network(&self, first: bool) {
        let info = super::network::info();
        let now = Instant::now();
        let was = self.snapshot().network.internet;
        let alert = {
            let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
            match (was, info.internet) {
                (Internet::Online, Internet::Offline | Internet::Limited) if !first => {
                    inner.offline_since = Some(now);
                    Some(SystemAlert::InternetDown)
                }
                (Internet::Offline | Internet::Limited, Internet::Online) => {
                    inner.offline_since.take().map(|since| SystemAlert::InternetBack {
                        seconds: now.duration_since(since).as_secs() as u32,
                    })
                }
                _ => None,
            }
        };
        self.update(|s| s.network = info);
        if let Some(alert) = alert.filter(|_| !self.hud().hide_internet) {
            self.alert(alert);
        }
    }

    #[cfg(windows)]
    fn refresh_radios(&self) {
        let radios = super::radios::read();
        self.update(|s| s.radios = radios);
    }

    #[cfg(windows)]
    fn refresh_power(&self, first: bool) -> Option<super::PowerInfo> {
        let power = super::battery::read();
        let was_plugged = self.snapshot().power.map(|p| p.plugged);
        if let Some(p) = &power {
            if !first && was_plugged == Some(false) && p.plugged && !self.hud().hide_charging {
                self.alert(SystemAlert::Plugged {
                    percent: p.percent,
                    minutes: p.minutes,
                });
            }
        }
        self.update(|s| s.power = power.clone());
        power
    }

    #[cfg(windows)]
    fn refresh_devices(&self, _first: bool) -> Vec<DeviceBattery> {
        let mut devices: Vec<DeviceBattery> = self
            .app
            .state::<crate::system::service::SystemService>()
            .bluetooth()
            .into_iter()
            .filter_map(|d| {
                Some(DeviceBattery {
                    name: d.name,
                    kind: "bluetooth".into(),
                    percent: d.battery?,
                })
            })
            .collect();
        devices.extend(super::gamepad::read());
        devices.sort_by(|a, b| a.name.cmp(&b.name));
        self.update(|s| s.devices = devices.clone());
        devices
    }

    #[cfg(windows)]
    fn slow_loop(&self) {
        let mut charge = ChargeWatch::default();
        let mut low = LowWatch::default();
        let mut ticks = 0u64;
        loop {
            let hud = self.hud();
            if let Some(p) = self.refresh_power(false) {
                let limit = if hud.no_charge_alert {
                    0
                } else if hud.charge_alert_at == 0 {
                    DEFAULT_CHARGE_ALERT
                } else {
                    hud.charge_alert_at
                };
                if charge.check(p.percent, p.plugged, limit) {
                    self.alert(SystemAlert::ChargeLimit { percent: p.percent });
                }
            }
            // Accessories every minute.
            if ticks.is_multiple_of(4) {
                let devices = self.refresh_devices(false);
                for d in low.check(&devices) {
                    if !hud.hide_device_low {
                        self.alert(SystemAlert::DeviceLow {
                            name: d.name,
                            percent: d.percent,
                        });
                    }
                }
            }
            // Network events can be missed (sleep, driver resets): re-read now and then.
            if ticks.is_multiple_of(4) {
                self.refresh_network(false);
            }
            ticks += 1;
            std::thread::sleep(SLOW_TICK);
        }
    }

    #[cfg(windows)]
    fn key_loop(&self) {
        let mut last = super::keys::read();
        loop {
            std::thread::sleep(KEY_TICK);
            let now = super::keys::read();
            if now == last {
                continue;
            }
            let hud = self.hud();
            if !hud.hide_caps && now.caps != last.caps {
                self.alert(SystemAlert::CapsLock { on: now.caps });
            } else if !hud.hide_caps && now.num != last.num {
                self.alert(SystemAlert::NumLock { on: now.num });
            }
            // Only a switch in the same window: apps can each have their own language.
            if !hud.hide_language && now.language != last.language && now.front == last.front {
                if let Some(name) = super::keys::language_name(now.language) {
                    self.alert(SystemAlert::Language { name });
                }
            }
            last = now;
        }
    }
}
