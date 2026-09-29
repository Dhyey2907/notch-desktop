//! Spotify connection state, playlist cache, and playback requests.

use std::{
    sync::{Arc, Mutex},
    time::Instant,
};

use tauri::{AppHandle, Manager};
use tauri_plugin_opener::OpenerExt;

use super::{
    authorization_url, exchange_code, me, play_playlist, playlist_app_uri, playlists, refresh, valid_id,
    ApiError, PlayOutcome, SpotifyErrorKind, SpotifyPlaylist, SpotifySnapshot, SpotifyStatus, MAX_PINNED,
    REDIRECT_PATH, REDIRECT_PORT,
};
use crate::{
    commands::settings::persist_and_broadcast,
    error::{AppError, AppResult},
    events::{AppEvent, EventEnvelope},
    oauth::{pkce_pair, Callback, Loopback, SIGN_IN_TIMEOUT},
    secrets::{self, Credential},
    settings::Settings,
    state::AppState,
};

const CACHE_KEY: &str = "spotify.playlists";

struct State {
    status: SpotifyStatus,
    playlists: Vec<SpotifyPlaylist>,
    premium: Option<bool>,
    access_token: Option<(String, Instant)>,
}

struct Inner {
    app: AppHandle,
    http: reqwest::Client,
    state: Mutex<State>,
    mock: bool,
}

#[derive(Clone)]
pub struct SpotifyService {
    inner: Arc<Inner>,
}

fn settings_of(app: &AppHandle) -> Settings {
    let state = app.state::<AppState>();
    let settings = state.settings.lock().unwrap_or_else(|e| e.into_inner()).clone();
    settings
}

fn update_settings(app: &AppHandle, change: impl FnOnce(&mut Settings)) -> AppResult<()> {
    let state = app.state::<AppState>();
    let mut next = settings_of(app);
    change(&mut next);
    persist_and_broadcast(&state, next).map(|_| ())
}

fn api_error(error: ApiError) -> AppError {
    match error {
        ApiError::Unauthorized => {
            AppError::InvalidInput("Spotify access expired; reconnect in Settings".into())
        }
        ApiError::NoActiveDevice => AppError::InvalidInput("no Spotify device is available".into()),
        ApiError::Forbidden => AppError::InvalidInput("Spotify Premium is required to start playback".into()),
        ApiError::Network(detail) => AppError::Platform(format!("network: {detail}")),
        ApiError::Other(detail) => AppError::Platform(detail),
    }
}

impl SpotifyService {
    pub fn new(app: AppHandle, mock: bool) -> Self {
        let cached: Vec<SpotifyPlaylist> = app
            .state::<AppState>()
            .db
            .with(|conn| {
                conn.query_row("SELECT value FROM kv_cache WHERE key = ?1", [CACHE_KEY], |row| {
                    row.get::<_, String>(0)
                })
                .map_err(Into::into)
            })
            .ok()
            .and_then(|json| serde_json::from_str(&json).ok())
            .unwrap_or_default();

        let connected = mock || matches!(secrets::get(Credential::SpotifyRefreshToken), Ok(Some(_)));
        let status = if connected {
            SpotifyStatus::Connected
        } else if settings_of(&app).spotify.client_id.is_some() {
            SpotifyStatus::Disconnected
        } else {
            SpotifyStatus::NotConfigured
        };
        Self {
            inner: Arc::new(Inner {
                app,
                http: reqwest::Client::builder()
                    .timeout(std::time::Duration::from_secs(15))
                    .build()
                    .expect("static client config"),
                state: Mutex::new(State {
                    status,
                    playlists: if connected { cached } else { Vec::new() },
                    premium: mock.then_some(true),
                    access_token: None,
                }),
                mock,
            }),
        }
    }

    /// Refresh playlists in the background at startup when already connected.
    pub fn start(&self) {
        if self.inner.lock().status != SpotifyStatus::Connected {
            return;
        }
        let this = self.clone();
        tauri::async_runtime::spawn(async move {
            if let Err(error) = this.refresh_playlists().await {
                tracing::info!(kind = error.kind(), "spotify playlist refresh failed at startup");
            }
        });
    }

    pub fn snapshot(&self) -> SpotifySnapshot {
        self.inner.snapshot()
    }

    pub async fn connect(&self, client_id: String) -> AppResult<()> {
        let client_id = client_id.trim().to_owned();
        if client_id.len() != 32 || !client_id.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(AppError::InvalidInput(
                "that doesn't look like a Spotify client ID (32 letters and numbers)".into(),
            ));
        }
        let saved = client_id.clone();
        update_settings(&self.inner.app, |s| s.spotify.client_id = Some(saved))?;
        self.inner.set_status(SpotifyStatus::Connecting);
        let result = self.sign_in(&client_id).await;
        match &result {
            Ok(()) => {
                self.inner.set_status(SpotifyStatus::Connected);
                if let Err(error) = self.refresh_playlists().await {
                    tracing::info!(
                        kind = error.kind(),
                        "spotify playlist refresh after connect failed"
                    );
                }
            }
            Err(error) => {
                tracing::info!(kind = error.kind(), "spotify sign-in did not complete");
                self.inner.set_status(SpotifyStatus::Disconnected);
            }
        }
        result
    }

    async fn sign_in(&self, client_id: &str) -> AppResult<()> {
        let loopback = Loopback::bind(REDIRECT_PORT, REDIRECT_PATH)?;
        let redirect_uri = loopback.redirect_uri()?;
        let (verifier, challenge) = pkce_pair();
        let state_token = uuid::Uuid::new_v4().to_string();
        self.inner
            .app
            .opener()
            .open_url(
                authorization_url(client_id, &redirect_uri, &challenge, &state_token),
                None::<&str>,
            )
            .map_err(|e| AppError::Platform(e.to_string()))?;
        let callback = tauri::async_runtime::spawn_blocking(move || {
            loopback.wait(&state_token, SIGN_IN_TIMEOUT, "Spotify")
        })
        .await
        .map_err(|e| AppError::Platform(e.to_string()))??;
        let code = match callback {
            Callback::Code(code) => code,
            Callback::Denied(reason) => {
                return Err(AppError::InvalidInput(format!("sign-in declined ({reason})")))
            }
        };
        let tokens = exchange_code(&self.inner.http, client_id, &code, &verifier, &redirect_uri)
            .await
            .map_err(api_error)?;
        let refresh_token = tokens
            .refresh_token
            .ok_or_else(|| AppError::Platform("Spotify did not return a refresh token".into()))?;
        secrets::set(Credential::SpotifyRefreshToken, &refresh_token)?;
        self.inner
            .store_access_token(tokens.access_token.clone(), tokens.expires_in);

        let (account, premium) = me(&self.inner.http, &tokens.access_token)
            .await
            .unwrap_or((None, None));
        self.inner.lock().premium = premium;
        update_settings(&self.inner.app, |s| s.spotify.account_name = account)?;
        tracing::info!("spotify connected");
        Ok(())
    }

    pub async fn disconnect(&self) -> AppResult<()> {
        // Spotify has no token revocation endpoint; access is removed from the user's account
        // page. Locally we forget the token and cached playlists.
        secrets::delete(Credential::SpotifyRefreshToken)?;
        self.inner.app.state::<AppState>().db.with(|conn| {
            conn.execute("DELETE FROM kv_cache WHERE key = ?1", [CACHE_KEY])?;
            Ok(())
        })?;
        update_settings(&self.inner.app, |s| s.spotify.account_name = None)?;
        {
            let mut state = self.inner.lock();
            state.playlists.clear();
            state.access_token = None;
            state.premium = None;
        }
        self.inner.set_status(SpotifyStatus::Disconnected);
        tracing::info!("spotify disconnected");
        Ok(())
    }

    pub async fn refresh_playlists(&self) -> AppResult<()> {
        let fetched = if self.inner.mock {
            Ok(mock_playlists())
        } else {
            match self.inner.token().await {
                Ok(token) => playlists(&self.inner.http, &token).await,
                Err(e) => Err(e),
            }
        };
        match fetched {
            Ok(list) => {
                let json = serde_json::to_string(&list).unwrap_or_else(|_| "[]".into());
                let saved = self.inner.app.state::<AppState>().db.with(|conn| {
                    conn.execute(
                        "INSERT INTO kv_cache (key, value, fetched_at) VALUES (?1, ?2, ?3)
                         ON CONFLICT(key) DO UPDATE SET value = excluded.value, fetched_at = excluded.fetched_at",
                        rusqlite::params![CACHE_KEY, json, chrono::Utc::now().to_rfc3339()],
                    )?;
                    Ok(())
                });
                if let Err(error) = saved {
                    tracing::warn!(%error, "could not cache spotify playlists");
                }
                {
                    let mut state = self.inner.lock();
                    state.playlists = list;
                    state.status = SpotifyStatus::Connected;
                }
                self.inner.publish();
                Ok(())
            }
            Err(ApiError::Unauthorized) => {
                self.inner.lock().access_token = None;
                self.inner.set_status(SpotifyStatus::Error {
                    reason: SpotifyErrorKind::AuthExpired,
                });
                Err(api_error(ApiError::Unauthorized))
            }
            Err(ApiError::Network(detail)) => {
                self.inner.set_status(SpotifyStatus::Error {
                    reason: SpotifyErrorKind::Network,
                });
                Err(api_error(ApiError::Network(detail)))
            }
            Err(other) => Err(api_error(other)),
        }
    }

    /// Start a playlist. Falls back to opening it in the Spotify app when there's no device to
    /// play on or the account can't start playback remotely.
    pub async fn play(&self, playlist_id: &str) -> AppResult<PlayOutcome> {
        if !valid_id(playlist_id) {
            return Err(AppError::InvalidInput("invalid playlist id".into()));
        }
        if self.inner.mock {
            tracing::info!("mock spotify: play requested");
            return Ok(PlayOutcome::Playing);
        }
        let token = self.inner.token().await.map_err(api_error)?;
        match play_playlist(&self.inner.http, &token, playlist_id).await {
            Ok(()) => Ok(PlayOutcome::Playing),
            Err(ApiError::NoActiveDevice | ApiError::Forbidden) => {
                self.inner
                    .app
                    .opener()
                    .open_url(playlist_app_uri(playlist_id), None::<&str>)
                    .map_err(|e| AppError::Platform(e.to_string()))?;
                Ok(PlayOutcome::OpenedInApp)
            }
            Err(ApiError::Unauthorized) => {
                self.inner.lock().access_token = None;
                self.inner.set_status(SpotifyStatus::Error {
                    reason: SpotifyErrorKind::AuthExpired,
                });
                Err(api_error(ApiError::Unauthorized))
            }
            Err(other) => Err(api_error(other)),
        }
    }

    pub fn set_pinned(&self, ids: Vec<String>) -> AppResult<()> {
        if ids.len() > MAX_PINNED || !ids.iter().all(|id| valid_id(id)) {
            return Err(AppError::InvalidInput("pick up to four playlists".into()));
        }
        update_settings(&self.inner.app, |s| s.spotify.pinned_playlists = ids)?;
        self.inner.publish();
        Ok(())
    }
}

impl Inner {
    fn lock(&self) -> std::sync::MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn snapshot(&self) -> SpotifySnapshot {
        let settings = settings_of(&self.app).spotify;
        let state = self.lock();
        SpotifySnapshot {
            status: state.status.clone(),
            account: if self.mock {
                Some("Mock listener".into())
            } else {
                settings.account_name
            },
            premium: state.premium,
            playlists: state.playlists.clone(),
            pinned: settings.pinned_playlists,
            mock: self.mock,
        }
    }

    fn publish(&self) {
        let envelope = EventEnvelope::new("spotify", AppEvent::SpotifyChanged(self.snapshot()))
            .with_dedup_key("spotify-snapshot");
        self.app.state::<AppState>().bus.publish(envelope);
    }

    fn set_status(&self, status: SpotifyStatus) {
        self.lock().status = status;
        self.publish();
    }

    fn store_access_token(&self, token: String, expires_in: u64) {
        let expires = Instant::now() + std::time::Duration::from_secs(expires_in.saturating_sub(60));
        self.lock().access_token = Some((token, expires));
    }

    async fn token(&self) -> Result<String, ApiError> {
        if let Some((token, expires)) = self.lock().access_token.clone() {
            if Instant::now() < expires {
                return Ok(token);
            }
        }
        let client_id = settings_of(&self.app)
            .spotify
            .client_id
            .ok_or(ApiError::Unauthorized)?;
        let refresh_token = secrets::get(Credential::SpotifyRefreshToken)
            .ok()
            .flatten()
            .ok_or(ApiError::Unauthorized)?;
        let tokens = refresh(&self.http, &client_id, &refresh_token).await?;
        // Spotify may rotate the refresh token; keep the newest one.
        if let Some(rotated) = tokens.refresh_token.as_deref() {
            if let Err(error) = secrets::set(Credential::SpotifyRefreshToken, rotated) {
                tracing::warn!(kind = error.kind(), "could not store rotated spotify token");
            }
        }
        if self.lock().premium.is_none() {
            if let Ok((_, premium)) = me(&self.http, &tokens.access_token).await {
                self.lock().premium = premium;
            }
        }
        self.store_access_token(tokens.access_token.clone(), tokens.expires_in);
        Ok(tokens.access_token)
    }
}

/// Development sample playlists (`NOTCH_MOCK=spotify`), clearly named as mock data.
fn mock_playlists() -> Vec<SpotifyPlaylist> {
    [
        "Focus Flow",
        "Late Night Drive",
        "Gym Mix",
        "Chill Sunday",
        "Throwbacks",
    ]
    .iter()
    .enumerate()
    .map(|(i, name)| SpotifyPlaylist {
        id: format!("mock{i}"),
        name: format!("{name} (MOCK)"),
        image_url: None,
        track_count: 20 + i as u32 * 7,
        owner: "Mock listener".into(),
    })
    .collect()
}
