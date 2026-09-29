//! Owns the Action Center reader thread.
//!
//! Windows gives unpackaged apps no "notification added" event, and it doesn't raise folder
//! change events for its notification database either (NTFS defers them). What does change
//! within milliseconds is the database file's timestamp, so the reader checks that once a
//! second — a metadata lookup costing microseconds — and only reads Action Center (~50 ms of
//! CPU) when it changed.

use std::{
    path::PathBuf,
    sync::{mpsc, Arc, Mutex},
    time::Duration,
};

use tauri::{AppHandle, Manager};

use super::{PhoneAccess, PhoneSnapshot, Tracker};
use crate::{error::AppResult, events::AppEvent, state::AppState};

/// Requests arrive in bursts; read once they settle.
const SETTLE: Duration = Duration::from_millis(150);
/// How often the notification database's timestamp is checked (cheap metadata only).
const CHECK_EVERY: Duration = Duration::from_secs(1);
/// Extra reads after a change, since new notifications become readable a little after Windows
/// writes them. Bounded, and only ever started by a real change.
const FOLLOW_UPS: [Duration; 4] = [
    Duration::from_millis(400),
    Duration::from_millis(1000),
    Duration::from_millis(2500),
    Duration::from_millis(5000),
];
const PHONE_LINK_PACKAGE: &str = "Microsoft.YourPhone_8wekyb3d8bbwe";

enum Command {
    /// Re-read Action Center (and the access permission).
    Refresh,
    /// Remove a notification from Action Center.
    Dismiss(u32),
}

struct Inner {
    app: AppHandle,
    snapshot: Mutex<PhoneSnapshot>,
    commands: Mutex<Option<mpsc::Sender<Command>>>,
}

#[derive(Clone)]
pub struct PhoneService {
    inner: Arc<Inner>,
}

fn local_app_data() -> Option<PathBuf> {
    std::env::var_os("LOCALAPPDATA").map(PathBuf::from)
}

/// Timestamp + size of the notification database files; changes within milliseconds of a
/// notification arriving.
fn database_stamp() -> Option<(std::time::SystemTime, u64)> {
    let dir = local_app_data()?
        .join("Microsoft")
        .join("Windows")
        .join("Notifications");
    let wal = std::fs::metadata(dir.join("wpndatabase.db-wal")).ok();
    let db = std::fs::metadata(dir.join("wpndatabase.db")).ok();
    let newest = [&wal, &db]
        .iter()
        .filter_map(|m| m.as_ref()?.modified().ok())
        .max()?;
    let size = [&wal, &db]
        .iter()
        .filter_map(|m| m.as_ref().map(|m| m.len()))
        .sum();
    Some((newest, size))
}

fn phone_link_installed() -> bool {
    local_app_data().is_some_and(|dir| dir.join("Packages").join(PHONE_LINK_PACKAGE).is_dir())
}

/// Which apps' notifications count as "phone". Dev builds can include every app
/// (`NOTCH_PHONE_ALL_APPS=1`) to test with ordinary Windows notifications.
fn include_filter() -> fn(&str) -> bool {
    #[cfg(debug_assertions)]
    if std::env::var("NOTCH_PHONE_ALL_APPS").is_ok_and(|v| v == "1") {
        return |_| true;
    }
    super::is_phone_link
}

impl PhoneService {
    pub fn new(app: AppHandle) -> Self {
        Self {
            inner: Arc::new(Inner {
                app,
                snapshot: Mutex::new(PhoneSnapshot {
                    access: PhoneAccess::Unknown,
                    phone_link_installed: phone_link_installed(),
                    notifications: Vec::new(),
                }),
                commands: Mutex::new(None),
            }),
        }
    }

    pub fn start(&self) {
        let (tx, rx) = mpsc::channel();
        *self.inner.commands.lock().unwrap_or_else(|e| e.into_inner()) = Some(tx.clone());
        let inner = self.inner.clone();
        let started = std::thread::Builder::new()
            .name("phone-notifications".into())
            .spawn(move || reader::run(inner, rx));
        if started.is_err() {
            tracing::warn!("could not start the phone notification reader");
            return;
        }
        let _ = tx.send(Command::Refresh);
    }

    pub fn snapshot(&self) -> PhoneSnapshot {
        // Cheap way to pick up a permission change made in Windows Settings.
        self.inner.send(Command::Refresh);
        self.inner
            .snapshot
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    pub fn dismiss(&self, id: u32) {
        self.inner.send(Command::Dismiss(id));
    }

    /// Bring up Phone Link (replies happen there).
    pub fn open_phone_link(&self) -> AppResult<()> {
        std::process::Command::new("explorer.exe")
            .arg(format!("shell:AppsFolder\\{PHONE_LINK_PACKAGE}!App"))
            .spawn()?;
        Ok(())
    }

    /// Windows Settings → Privacy & security → Notifications (where access is granted).
    pub fn open_access_settings(&self) -> AppResult<()> {
        use tauri_plugin_opener::OpenerExt;
        self.inner
            .app
            .opener()
            .open_url("ms-settings:privacy-notifications", None::<&str>)
            .map_err(|e| crate::error::AppError::Platform(e.to_string()))
    }
}

impl Inner {
    fn send(&self, command: Command) {
        if let Some(tx) = self.commands.lock().unwrap_or_else(|e| e.into_inner()).as_ref() {
            let _ = tx.send(command);
        }
    }

    fn publish(&self) {
        let snapshot = self.snapshot.lock().unwrap_or_else(|e| e.into_inner()).clone();
        self.app
            .state::<AppState>()
            .bus
            .emit("phone", AppEvent::PhoneChanged(snapshot));
    }
}

#[cfg(windows)]
mod reader {
    use std::sync::{mpsc, Arc};

    use chrono::{DateTime, TimeZone, Utc};
    use tauri::Manager;
    use windows::{
        Win32::System::WinRT::{RoInitialize, RO_INIT_MULTITHREADED},
        UI::Notifications::{
            KnownNotificationBindings,
            Management::{UserNotificationListener, UserNotificationListenerAccessStatus},
            NotificationKinds, UserNotification,
        },
    };

    use std::time::Instant;

    use super::{include_filter, Command, Inner, PhoneAccess, Tracker, CHECK_EVERY, FOLLOW_UPS, SETTLE};
    use crate::{events::AppEvent, phone::RawNotification, state::AppState};

    /// WinRT `DateTime` is 100 ns ticks since 1601-01-01.
    fn to_utc(ticks: i64) -> DateTime<Utc> {
        const TICKS_TO_UNIX_EPOCH: i64 = 116_444_736_000_000_000;
        let unix_100ns = ticks - TICKS_TO_UNIX_EPOCH;
        Utc.timestamp_opt(
            unix_100ns.div_euclid(10_000_000),
            (unix_100ns.rem_euclid(10_000_000) * 100) as u32,
        )
        .single()
        .unwrap_or_default()
    }

    fn read(n: &UserNotification) -> Option<RawNotification> {
        let app_id = n.AppInfo().ok()?.AppUserModelId().ok()?.to_string();
        let texts = n
            .Notification()
            .and_then(|x| x.Visual())
            .and_then(|v| v.GetBinding(&KnownNotificationBindings::ToastGeneric()?))
            .and_then(|b| b.GetTextElements())
            .map(|elements| {
                elements
                    .into_iter()
                    .filter_map(|t| t.Text().ok().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();
        Some(RawNotification {
            id: n.Id().ok()?,
            app_id,
            texts,
            at: to_utc(n.CreationTime().ok()?.UniversalTime),
        })
    }

    pub(super) fn run(inner: Arc<Inner>, rx: mpsc::Receiver<Command>) {
        // SAFETY: initializes WinRT for this dedicated thread only.
        let _ = unsafe { RoInitialize(RO_INIT_MULTITHREADED) };
        let Ok(listener) = UserNotificationListener::Current() else {
            tracing::warn!("notification listener unavailable");
            return;
        };
        let mut tracker = Tracker::new(include_filter());
        // Pending extra reads after a change (see FOLLOW_UPS).
        let mut follow_ups: Vec<Instant> = Vec::new();
        let mut stamp = super::database_stamp();
        let mut next_check = Instant::now() + CHECK_EVERY;

        loop {
            // Wait for a request, the next follow-up read, or the next timestamp check.
            let deadline = follow_ups.first().map_or(next_check, |at| (*at).min(next_check));
            let woke = match rx.recv_timeout(deadline.saturating_duration_since(Instant::now())) {
                Ok(command) => Some(command),
                Err(mpsc::RecvTimeoutError::Timeout) => None,
                Err(mpsc::RecvTimeoutError::Disconnected) => return,
            };
            let mut commands = Vec::new();
            let mut changed = false;
            if let Some(first) = woke {
                // Coalesce a burst of requests into one read.
                commands.push(first);
                while let Ok(more) = rx.recv_timeout(SETTLE) {
                    commands.push(more);
                }
                changed = true;
            }
            let now = Instant::now();
            if now >= next_check {
                next_check = now + CHECK_EVERY;
                let current = super::database_stamp();
                if current != stamp {
                    stamp = current;
                    changed = true;
                }
            }
            let mut due = false;
            while follow_ups.first().is_some_and(|at| *at <= now) {
                follow_ups.remove(0);
                due = true;
            }
            if changed {
                // Windows makes a new notification readable a moment after writing it to
                // its database, so the first read can miss it: look again a few times.
                follow_ups = FOLLOW_UPS.iter().map(|delay| now + *delay).collect();
            } else if !due {
                continue;
            }
            let mut hid = false;
            for command in &commands {
                if let Command::Dismiss(id) = command {
                    // Ignored by Windows for unregistered apps; hide it in Notch either way.
                    let _ = listener.RemoveNotification(*id);
                    hid |= tracker.hide(*id);
                }
            }

            let access = match listener.GetAccessStatus() {
                Ok(UserNotificationListenerAccessStatus::Allowed) => PhoneAccess::Allowed,
                Ok(UserNotificationListenerAccessStatus::Denied) => PhoneAccess::Denied,
                _ => PhoneAccess::Unknown,
            };
            let current: Vec<RawNotification> = if access == PhoneAccess::Allowed {
                match listener
                    .GetNotificationsAsync(NotificationKinds::Toast)
                    .and_then(|op| op.get())
                {
                    Ok(list) => list.into_iter().filter_map(|n| read(&n)).collect(),
                    Err(error) => {
                        tracing::debug!(error = %error.message(), "could not read notifications");
                        continue;
                    }
                }
            } else {
                Vec::new()
            };

            let (arrived, changed) = tracker.update(&current);
            let access_changed = {
                let mut snapshot = inner.snapshot.lock().unwrap_or_else(|e| e.into_inner());
                let access_changed = snapshot.access != access;
                snapshot.access = access;
                snapshot.phone_link_installed = super::phone_link_installed();
                snapshot.notifications = tracker.list().to_vec();
                access_changed
            };
            for notification in arrived {
                // Only the fact that one arrived is logged, never its content.
                tracing::debug!("phone notification arrived");
                inner
                    .app
                    .state::<AppState>()
                    .bus
                    .emit("phone", AppEvent::PhoneNotification(notification));
            }
            if changed || access_changed || hid {
                inner.publish();
            }
        }
    }
}

#[cfg(not(windows))]
mod reader {
    use std::sync::{mpsc, Arc};

    use super::{Command, Inner};

    pub(super) fn run(_inner: Arc<Inner>, _rx: mpsc::Receiver<Command>) {}
}
