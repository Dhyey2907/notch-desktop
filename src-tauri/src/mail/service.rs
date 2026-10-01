//! Checks Gmail every two minutes while it's turned on and allowed; new unread mail gets a card.

use std::{
    collections::HashSet,
    sync::{Arc, Mutex},
    time::Duration,
};

use serde_json::Value;
use tauri::{AppHandle, Manager};

use super::{parse_message, MailMessage, MailState, MailStatus};
use crate::{
    calendar::{oauth::GMAIL_SCOPE, service::CalendarService},
    error::{AppError, AppResult},
    events::AppEvent,
    state::AppState,
};

const API: &str = "https://gmail.googleapis.com/gmail/v1/users/me";
const EVERY: Duration = Duration::from_secs(120);
const SHOWN: usize = 8;
/// At most this many cards for one check (a busy inbox shouldn't flood the notch).
const MAX_CARDS: usize = 3;

struct Inner {
    state: MailState,
    /// Unread messages already seen (no card for them again).
    seen: HashSet<String>,
    first: bool,
}

#[derive(Clone)]
pub struct MailService {
    app: AppHandle,
    http: reqwest::Client,
    inner: Arc<Mutex<Inner>>,
    wake: Arc<tokio::sync::Notify>,
}

impl MailService {
    pub fn new(app: AppHandle) -> Self {
        Self {
            app,
            http: reqwest::Client::builder()
                .timeout(Duration::from_secs(20))
                .user_agent("Notch-Desktop")
                .build()
                .unwrap_or_default(),
            inner: Arc::new(Mutex::new(Inner {
                state: MailState {
                    status: MailStatus::Off,
                    unread: 0,
                    messages: Vec::new(),
                },
                seen: HashSet::new(),
                first: true,
            })),
            wake: Arc::default(),
        }
    }

    pub fn state(&self) -> MailState {
        self.inner.lock().unwrap_or_else(|e| e.into_inner()).state.clone()
    }

    fn set(&self, state: MailState) {
        let changed = {
            let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
            let changed = inner.state != state;
            inner.state = state.clone();
            changed
        };
        if changed {
            self.app
                .state::<AppState>()
                .bus
                .emit("mail", AppEvent::MailChanged(state));
        }
    }

    fn calendar(&self) -> CalendarService {
        self.app.state::<CalendarService>().inner().clone()
    }

    pub fn start(&self) {
        let this = self.clone();
        tauri::async_runtime::spawn(async move {
            // Also check again whenever settings change (Gmail turned on, signed in again).
            let wake = this.wake.clone();
            let app = this.app.clone();
            tauri::async_runtime::spawn(async move {
                let mut events = app.state::<AppState>().bus.subscribe();
                while let Ok(envelope) = events.recv().await {
                    if matches!(envelope.event, AppEvent::SettingsChanged(_)) {
                        wake.notify_one();
                    }
                }
            });
            loop {
                this.check().await;
                tokio::select! {
                    _ = tokio::time::sleep(EVERY) => {}
                    _ = this.wake.notified() => {}
                }
            }
        });
    }

    async fn get(&self, url: &str) -> AppResult<Value> {
        let token = self.calendar().google_token().await?;
        let response = self
            .http
            .get(url)
            .bearer_auth(token)
            .send()
            .await
            .map_err(|_| AppError::InvalidInput("You're offline".into()))?;
        if !response.status().is_success() {
            return Err(AppError::Platform(format!(
                "gmail answered {}",
                response.status().as_u16()
            )));
        }
        response
            .json()
            .await
            .map_err(|e| AppError::Platform(e.to_string()))
    }

    async fn check(&self) {
        let enabled = self
            .app
            .state::<AppState>()
            .settings
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .calendar
            .gmail;
        if !enabled {
            return self.set(MailState {
                status: MailStatus::Off,
                unread: 0,
                messages: Vec::new(),
            });
        }
        if !self.calendar().granted(GMAIL_SCOPE) {
            return self.set(MailState {
                status: MailStatus::NeedsSignIn,
                unread: 0,
                messages: Vec::new(),
            });
        }
        match self.fetch().await {
            Ok((unread, messages)) => {
                let fresh: Vec<MailMessage> = {
                    let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
                    let fresh = if inner.first {
                        Vec::new()
                    } else {
                        messages
                            .iter()
                            .filter(|m| !inner.seen.contains(&m.id))
                            .cloned()
                            .collect()
                    };
                    inner.first = false;
                    inner.seen = messages.iter().map(|m| m.id.clone()).collect();
                    fresh
                };
                self.set(MailState {
                    status: MailStatus::Ready,
                    unread,
                    messages,
                });
                for m in fresh.into_iter().take(MAX_CARDS) {
                    self.app
                        .state::<AppState>()
                        .bus
                        .emit("mail", AppEvent::NewMail(m));
                }
            }
            Err(error) => {
                tracing::info!(kind = error.kind(), "gmail check failed");
                let mut state = self.state();
                state.status = MailStatus::Error;
                self.set(state);
            }
        }
    }

    async fn fetch(&self) -> AppResult<(u32, Vec<MailMessage>)> {
        let inbox = self.get(&format!("{API}/labels/INBOX")).await?;
        let unread = inbox["messagesUnread"].as_u64().unwrap_or(0) as u32;
        let list = self
            .get(&format!(
                "{API}/messages?q=is%3Aunread%20in%3Ainbox&maxResults={SHOWN}"
            ))
            .await?;
        let mut messages = Vec::new();
        for item in list["messages"].as_array().cloned().unwrap_or_default() {
            let Some(id) = item["id"].as_str() else { continue };
            let json = self
                .get(&format!(
                    "{API}/messages/{id}?format=metadata&metadataHeaders=From&metadataHeaders=Subject"
                ))
                .await?;
            if let Some(m) = parse_message(&json) {
                messages.push(m);
            }
        }
        Ok((unread, messages))
    }

    async fn modify(&self, id: &str, remove: &str) -> AppResult<()> {
        if id.is_empty() || id.len() > 64 || !id.chars().all(|c| c.is_ascii_alphanumeric()) {
            return Err(AppError::InvalidInput("invalid message".into()));
        }
        let token = self.calendar().google_token().await?;
        let response = self
            .http
            .post(format!("{API}/messages/{id}/modify"))
            .bearer_auth(token)
            .json(&serde_json::json!({ "removeLabelIds": [remove] }))
            .send()
            .await
            .map_err(|_| AppError::InvalidInput("You're offline".into()))?;
        if !response.status().is_success() {
            return Err(AppError::InvalidInput("Gmail didn't accept that".into()));
        }
        self.wake.notify_one();
        Ok(())
    }

    pub async fn archive(&self, id: &str) -> AppResult<()> {
        self.modify(id, "INBOX").await
    }

    pub async fn mark_read(&self, id: &str) -> AppResult<()> {
        self.modify(id, "UNREAD").await
    }

    pub fn refresh(&self) {
        self.wake.notify_one();
    }
}
