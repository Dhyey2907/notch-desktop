//! Calendar sync, cache, and reminders.
//!
//! One background task sleeps until the next thing it must do — a reminder, the periodic
//! 15-minute sync, or a nudge (connect, "sync now", settings change) — so there is no tight
//! polling (plan.md Rule 7). Google offers push notifications only to public webhooks, which
//! a desktop app can't receive, hence the modest periodic sync.

use std::{
    collections::HashSet,
    sync::{Arc, Mutex},
    time::Instant,
};

use chrono::{DateTime, Duration, Utc};
use tauri::{AppHandle, Manager};
use tauri_plugin_opener::OpenerExt;
use tokio::sync::Notify;

use super::{
    google::{self, FetchError},
    oauth::{self, TokenFailure},
    reminder_key, reminders, sync_window, CalendarErrorKind, CalendarEvent, CalendarSnapshot, CalendarStatus,
};
use crate::{
    commands::settings::persist_and_broadcast,
    error::{AppError, AppResult},
    events::{AppEvent, EventEnvelope},
    oauth::{Callback, Loopback, SIGN_IN_TIMEOUT},
    secrets::{self, Credential},
    state::AppState,
};

const SYNC_INTERVAL_MIN: i64 = 15;
/// Upper bound on how long the task sleeps, so reminders stay on time after the PC wakes
/// from sleep (the monotonic clock doesn't advance while suspended).
const MAX_SLEEP_SECS: i64 = 60;
const CACHE_KEY: &str = "calendar.events";

struct State {
    status: CalendarStatus,
    events: Vec<CalendarEvent>,
    last_synced_at: Option<DateTime<Utc>>,
    access_token: Option<(String, Instant)>,
    fired: HashSet<String>,
    consecutive_failures: u32,
    next_sync_at: Option<DateTime<Utc>>,
}

struct Inner {
    app: AppHandle,
    http: reqwest::Client,
    state: Mutex<State>,
    wake: Notify,
    mock: bool,
}

#[derive(Clone)]
pub struct CalendarService {
    inner: Arc<Inner>,
}

enum SyncError {
    AuthExpired,
    Network,
    Other,
}

fn settings_of(app: &AppHandle) -> crate::settings::Settings {
    let state = app.state::<AppState>();
    let settings = state.settings.lock().unwrap_or_else(|e| e.into_inner()).clone();
    settings
}

fn update_settings(app: &AppHandle, change: impl FnOnce(&mut crate::settings::Settings)) -> AppResult<()> {
    let state = app.state::<AppState>();
    let mut next = settings_of(app);
    change(&mut next);
    persist_and_broadcast(&state, next).map(|_| ())
}

/// Exponential backoff after failed syncs: 1, 2, 4, 8, then every 15 minutes.
fn backoff(failures: u32) -> Duration {
    Duration::minutes((1_i64 << failures.saturating_sub(1).min(4)).min(SYNC_INTERVAL_MIN))
}

impl CalendarService {
    pub fn new(app: AppHandle, mock: bool) -> Self {
        let cached = app
            .state::<AppState>()
            .db
            .with(|conn| {
                conn.query_row(
                    "SELECT value, fetched_at FROM kv_cache WHERE key = ?1",
                    [CACHE_KEY],
                    |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
                )
                .map_err(Into::into)
            })
            .ok();
        let (events, last_synced_at) = cached
            .map(|(json, at)| {
                (
                    serde_json::from_str::<Vec<CalendarEvent>>(&json).unwrap_or_default(),
                    DateTime::parse_from_rfc3339(&at)
                        .ok()
                        .map(|t| t.with_timezone(&Utc)),
                )
            })
            .unwrap_or_default();

        let settings = settings_of(&app);
        let status = if mock || matches!(secrets::get(Credential::GoogleRefreshToken), Ok(Some(_))) {
            CalendarStatus::Connected
        } else if settings.calendar.client_id.is_some() {
            CalendarStatus::Disconnected
        } else {
            CalendarStatus::NotConfigured
        };
        let connected = status == CalendarStatus::Connected;

        Self {
            inner: Arc::new(Inner {
                app,
                http: reqwest::Client::builder()
                    .timeout(std::time::Duration::from_secs(20))
                    .build()
                    .expect("static client config"),
                state: Mutex::new(State {
                    status,
                    events: if connected { events } else { Vec::new() },
                    last_synced_at: if connected { last_synced_at } else { None },
                    access_token: None,
                    fired: HashSet::new(),
                    consecutive_failures: 0,
                    next_sync_at: connected.then(Utc::now),
                }),
                wake: Notify::new(),
                mock,
            }),
        }
    }

    pub fn start(&self) {
        let inner = self.inner.clone();
        tauri::async_runtime::spawn(async move { inner.run().await });
    }

    pub fn snapshot(&self) -> CalendarSnapshot {
        self.inner.snapshot()
    }

    pub fn sync_now(&self) {
        let mut state = self.inner.lock();
        if matches!(
            state.status,
            CalendarStatus::Connected | CalendarStatus::Error { .. }
        ) {
            state.next_sync_at = Some(Utc::now());
        }
        drop(state);
        self.inner.wake.notify_one();
    }

    /// Sign in with Google in the system browser and start syncing. Returns once the user
    /// has finished (or abandoned) sign-in.
    pub async fn connect(&self, client_id: String, client_secret: String) -> AppResult<()> {
        let client_id = client_id.trim().to_owned();
        let client_secret = client_secret.trim().to_owned();
        if !client_id.ends_with(".apps.googleusercontent.com") || client_id.len() > 200 {
            return Err(AppError::InvalidInput(
                "that doesn't look like a Google OAuth client ID".into(),
            ));
        }
        // An empty secret means "keep the one already saved".
        if client_secret.is_empty() {
            if secrets::get(Credential::GoogleClientSecret)?.is_none() {
                return Err(AppError::InvalidInput("a client secret is required".into()));
            }
        } else if client_secret.len() > 200 {
            return Err(AppError::InvalidInput("client secret is too long".into()));
        } else {
            secrets::set(Credential::GoogleClientSecret, &client_secret)?;
        }
        let saved_id = client_id.clone();
        update_settings(&self.inner.app, |s| s.calendar.client_id = Some(saved_id))?;

        self.inner.set_status(CalendarStatus::Connecting);
        let result = self.inner.sign_in(&client_id).await;
        match &result {
            Ok(()) => {
                self.inner.lock().next_sync_at = Some(Utc::now());
                self.inner.set_status(CalendarStatus::Connected);
                self.inner.wake.notify_one();
            }
            Err(error) => {
                tracing::info!(kind = error.kind(), "calendar sign-in did not complete");
                self.inner.set_status(CalendarStatus::Disconnected);
            }
        }
        result
    }

    /// Revoke access (best effort), forget the refresh token, and clear cached events.
    /// The OAuth client ID/secret are kept so reconnecting is one click.
    pub async fn disconnect(&self) -> AppResult<()> {
        if let Ok(Some(token)) = secrets::get(Credential::GoogleRefreshToken) {
            oauth::revoke(&self.inner.http, &token).await;
        }
        secrets::delete(Credential::GoogleRefreshToken)?;
        self.inner.app.state::<AppState>().db.with(|conn| {
            conn.execute("DELETE FROM kv_cache WHERE key = ?1", [CACHE_KEY])?;
            Ok(())
        })?;
        update_settings(&self.inner.app, |s| s.calendar.account_email = None)?;
        {
            let mut state = self.inner.lock();
            state.events.clear();
            state.last_synced_at = None;
            state.access_token = None;
            state.next_sync_at = None;
            state.fired.clear();
        }
        self.inner.set_status(CalendarStatus::Disconnected);
        tracing::info!("calendar disconnected");
        Ok(())
    }

    /// A Google access token for other features (Gmail, Classroom) using the same sign-in.
    pub async fn google_token(&self) -> AppResult<String> {
        self.inner.access_token().await.map_err(|error| match error {
            SyncError::AuthExpired => {
                AppError::InvalidInput("Sign in with Google again in Settings → Calendar".into())
            }
            SyncError::Network => AppError::InvalidInput("You're offline".into()),
            _ => AppError::Platform("Google didn't answer".into()),
        })
    }

    /// Whether the user allowed this extra (Gmail, Classroom) when signing in.
    pub fn granted(&self, scope: &str) -> bool {
        let calendar = settings_of(&self.inner.app).calendar;
        calendar.account_email.is_some()
            && scope
                .split_whitespace()
                .all(|s| calendar.granted_scopes.iter().any(|g| g == s))
    }

    /// Today's events (already synced).
    pub fn events(&self) -> Vec<super::CalendarEvent> {
        self.snapshot().events
    }

    /// Open an event or meeting link in the default browser. Only https is allowed.
    pub fn open_link(&self, url: &str) -> AppResult<()> {
        let parsed = url::Url::parse(url).map_err(|_| AppError::InvalidInput("invalid link".into()))?;
        if parsed.scheme() != "https" {
            return Err(AppError::InvalidInput("only https links can be opened".into()));
        }
        self.inner
            .app
            .opener()
            .open_url(parsed.as_str(), None::<&str>)
            .map_err(|e| AppError::Platform(e.to_string()))
    }
}

impl Inner {
    fn lock(&self) -> std::sync::MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn snapshot(&self) -> CalendarSnapshot {
        let state = self.lock();
        CalendarSnapshot {
            status: state.status.clone(),
            account: if self.mock {
                Some("mock@example.com".into())
            } else {
                settings_of(&self.app).calendar.account_email
            },
            events: state.events.clone(),
            last_synced_at: state.last_synced_at,
            mock: self.mock,
        }
    }

    fn publish(&self) {
        let envelope = EventEnvelope::new("calendar", AppEvent::CalendarChanged(self.snapshot()))
            .with_dedup_key("calendar-snapshot");
        self.app.state::<AppState>().bus.publish(envelope);
    }

    fn set_status(&self, status: CalendarStatus) {
        self.lock().status = status;
        self.publish();
    }

    async fn sign_in(&self, client_id: &str) -> AppResult<()> {
        let client_secret = secrets::get(Credential::GoogleClientSecret)?
            .ok_or_else(|| AppError::InvalidInput("missing client secret".into()))?;
        let loopback = Loopback::bind(0, "/")?;
        let redirect_uri = loopback.redirect_uri()?;
        let (verifier, challenge) = crate::oauth::pkce_pair();
        let state_token = uuid::Uuid::new_v4().to_string();
        let calendar = settings_of(&self.app).calendar;
        let scopes = oauth::scopes_for(calendar.gmail, calendar.classroom);
        let url = oauth::authorization_url(client_id, &redirect_uri, &challenge, &state_token, &scopes);
        self.app
            .opener()
            .open_url(url, None::<&str>)
            .map_err(|e| AppError::Platform(e.to_string()))?;

        let callback = tauri::async_runtime::spawn_blocking(move || {
            loopback.wait(&state_token, SIGN_IN_TIMEOUT, "Calendar")
        })
        .await
        .map_err(|e| AppError::Platform(e.to_string()))??;
        let code = match callback {
            Callback::Code(code) => code,
            Callback::Denied(reason) => {
                return Err(AppError::InvalidInput(format!("sign-in declined ({reason})")))
            }
        };

        let tokens = oauth::exchange_code(
            &self.http,
            client_id,
            &client_secret,
            &code,
            &verifier,
            &redirect_uri,
        )
        .await
        .map_err(|failure| AppError::Platform(format!("token exchange failed: {failure:?}")))?;
        let refresh_token = tokens
            .refresh_token
            .ok_or_else(|| AppError::Platform("Google did not return a refresh token".into()))?;
        secrets::set(Credential::GoogleRefreshToken, &refresh_token)?;
        let email = tokens.id_token.as_deref().and_then(oauth::email_from_id_token);
        let granted: Vec<String> = tokens
            .scope
            .as_deref()
            .unwrap_or(&scopes)
            .split_whitespace()
            .map(str::to_owned)
            .collect();
        update_settings(&self.app, |s| {
            s.calendar.account_email = email;
            s.calendar.granted_scopes = granted;
        })?;
        self.lock().access_token = Some((
            tokens.access_token,
            Instant::now() + std::time::Duration::from_secs(tokens.expires_in.saturating_sub(60)),
        ));
        tracing::info!("calendar connected");
        Ok(())
    }

    async fn access_token(&self) -> Result<String, SyncError> {
        if let Some((token, expires)) = self.lock().access_token.clone() {
            if Instant::now() < expires {
                return Ok(token);
            }
        }
        let client_id = settings_of(&self.app)
            .calendar
            .client_id
            .ok_or(SyncError::AuthExpired)?;
        let secret = secrets::get(Credential::GoogleClientSecret)
            .ok()
            .flatten()
            .ok_or(SyncError::AuthExpired)?;
        let refresh_token = secrets::get(Credential::GoogleRefreshToken)
            .ok()
            .flatten()
            .ok_or(SyncError::AuthExpired)?;
        match oauth::refresh(&self.http, &client_id, &secret, &refresh_token).await {
            Ok(tokens) => {
                let expires =
                    Instant::now() + std::time::Duration::from_secs(tokens.expires_in.saturating_sub(60));
                self.lock().access_token = Some((tokens.access_token.clone(), expires));
                Ok(tokens.access_token)
            }
            Err(TokenFailure::InvalidGrant) => Err(SyncError::AuthExpired),
            Err(TokenFailure::Network(detail)) => {
                tracing::info!(%detail, "calendar token refresh: network error");
                Err(SyncError::Network)
            }
            Err(TokenFailure::Other(detail)) => {
                tracing::warn!(%detail, "calendar token refresh failed");
                Err(SyncError::Other)
            }
        }
    }

    async fn fetch(&self, start: DateTime<Utc>, end: DateTime<Utc>) -> Result<Vec<CalendarEvent>, SyncError> {
        if self.mock {
            return Ok(mock_events(Utc::now()));
        }
        for attempt in 0..2 {
            let token = self.access_token().await?;
            match google::fetch_events(&self.http, &token, start, end).await {
                Ok(events) => return Ok(events),
                // A stale access token: drop it and refresh once.
                Err(FetchError::Unauthorized) if attempt == 0 => self.lock().access_token = None,
                Err(FetchError::Unauthorized) => return Err(SyncError::AuthExpired),
                Err(FetchError::Network(detail)) => {
                    tracing::info!(%detail, "calendar fetch: network error");
                    return Err(SyncError::Network);
                }
                Err(FetchError::Other(detail)) => {
                    tracing::warn!(%detail, "calendar fetch failed");
                    return Err(SyncError::Other);
                }
            }
        }
        Err(SyncError::AuthExpired)
    }

    async fn sync(&self) {
        let now = Utc::now();
        let (start, end) = sync_window(now);
        let result = self.fetch(start, end).await;
        {
            let mut state = self.lock();
            match result {
                Ok(events) => {
                    let count = events.len();
                    let json = serde_json::to_string(&events).unwrap_or_else(|_| "[]".into());
                    let saved = self.app.state::<AppState>().db.with(|conn| {
                        conn.execute(
                            "INSERT INTO kv_cache (key, value, fetched_at) VALUES (?1, ?2, ?3)
                             ON CONFLICT(key) DO UPDATE SET value = excluded.value, fetched_at = excluded.fetched_at",
                            rusqlite::params![CACHE_KEY, json, now.to_rfc3339()],
                        )?;
                        Ok(())
                    });
                    if let Err(error) = saved {
                        tracing::warn!(%error, "could not cache calendar events");
                    }
                    state.events = events;
                    state.last_synced_at = Some(now);
                    state.status = CalendarStatus::Connected;
                    state.consecutive_failures = 0;
                    state.next_sync_at = Some(now + Duration::minutes(SYNC_INTERVAL_MIN));
                    tracing::debug!(count, "calendar synced");
                }
                Err(SyncError::AuthExpired) => {
                    state.status = CalendarStatus::Error {
                        reason: CalendarErrorKind::AuthExpired,
                    };
                    state.access_token = None;
                    state.next_sync_at = None; // wait for the user to reconnect
                    tracing::warn!("calendar access expired or was revoked");
                }
                Err(kind) => {
                    state.consecutive_failures += 1;
                    let reason = match kind {
                        SyncError::Network => CalendarErrorKind::Network,
                        _ => CalendarErrorKind::Unknown,
                    };
                    state.status = CalendarStatus::Error { reason };
                    state.next_sync_at = Some(now + backoff(state.consecutive_failures));
                    tracing::info!(
                        failures = state.consecutive_failures,
                        "calendar sync failed; will retry"
                    );
                }
            }
        }
        self.publish();
    }

    async fn run(self: Arc<Self>) {
        let mut settings_changes = self.app.state::<AppState>().bus.subscribe();
        self.publish();
        loop {
            let now = Utc::now();
            let due_sync = self.lock().next_sync_at.is_some_and(|at| at <= now);
            if due_sync {
                self.sync().await;
            }

            let lead = Duration::minutes(i64::from(settings_of(&self.app).calendar.reminder_minutes));
            let (due, next_reminder) = {
                let mut state = self.lock();
                let (due, next) = if lead > Duration::zero() {
                    let (due, next) = reminders(&state.events, lead, now, &state.fired);
                    (due.into_iter().cloned().collect::<Vec<_>>(), next)
                } else {
                    (Vec::new(), None)
                };
                for event in &due {
                    state.fired.insert(reminder_key(event));
                }
                // Forget reminders for events that have ended.
                let live: HashSet<String> = state
                    .events
                    .iter()
                    .filter(|e| e.end > now)
                    .map(reminder_key)
                    .collect();
                state.fired.retain(|k| live.contains(k));
                (due, next)
            };
            for event in due {
                tracing::debug!("calendar reminder due");
                self.app
                    .state::<AppState>()
                    .bus
                    .emit("calendar", AppEvent::ReminderDue { event });
            }

            let next_sync = self.lock().next_sync_at;
            let wake_at = [
                next_reminder,
                next_sync,
                Some(now + Duration::seconds(MAX_SLEEP_SECS)),
            ]
            .into_iter()
            .flatten()
            .min()
            .unwrap_or(now);
            let sleep = (wake_at - Utc::now()).to_std().unwrap_or_default();
            tokio::select! {
                _ = tokio::time::sleep(sleep) => {}
                _ = self.wake.notified() => {}
                change = settings_changes.recv() => {
                    // Reminder lead time may have changed; loop around to recompute.
                    if matches!(change, Err(tokio::sync::broadcast::error::RecvError::Closed)) {
                        return;
                    }
                }
            }
        }
    }
}

/// Development sample events relative to now, including one whose reminder is due within
/// a minute so the countdown can be exercised. Titles are marked MOCK.
fn mock_events(now: DateTime<Utc>) -> Vec<CalendarEvent> {
    use std::sync::OnceLock;
    // Anchor once so repeated syncs don't keep moving the events.
    static ANCHOR: OnceLock<DateTime<Utc>> = OnceLock::new();
    let anchor = *ANCHOR.get_or_init(|| now);
    let (today, _) = sync_window(anchor);
    let at = |minutes: i64| anchor + Duration::minutes(minutes);
    let event = |id: &str, title: &str, start: DateTime<Utc>, minutes: i64, join: bool| CalendarEvent {
        id: id.into(),
        title: format!("{title} · MOCK"),
        start,
        end: start + Duration::minutes(minutes),
        all_day: false,
        join_url: join.then(|| "https://example.com/mock-meeting".into()),
        location: None,
        html_link: None,
    };
    let mut events = vec![
        CalendarEvent {
            all_day: true,
            ..event("mock-allday", "Team offsite", today, 24 * 60, false)
        },
        event("mock-soon", "Design Review", at(6), 30, true),
        event("mock-later", "1:1 with Sam", at(95), 30, true),
        event(
            "mock-tomorrow",
            "Planning",
            today + Duration::hours(24 + 10),
            60,
            false,
        ),
    ];
    events.sort_by_key(|e| e.start);
    events
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backoff_grows_then_caps_at_the_sync_interval() {
        let minutes: Vec<i64> = (1..=7).map(|n| backoff(n).num_minutes()).collect();
        assert_eq!(minutes, [1, 2, 4, 8, 15, 15, 15]);
    }

    #[test]
    fn mock_events_include_a_reminder_due_soon() {
        let now = Utc::now();
        let events = mock_events(now);
        let (due, next) = reminders(&events, Duration::minutes(5), now, &HashSet::new());
        assert!(due.is_empty());
        let next = next.unwrap();
        assert!(next > now && next <= now + Duration::minutes(2));
        assert!(events.iter().all(|e| e.title.ends_with("MOCK")));
    }
}
