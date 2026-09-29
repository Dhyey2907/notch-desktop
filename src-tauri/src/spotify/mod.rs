//! Spotify Web API integration for pinned playlists in the Media module.
//!
//! Sign-in uses Authorization Code + PKCE with the user's own Spotify app (client ID only,
//! no secret). Spotify requires the exact redirect URI to be registered, so the loopback
//! listener uses a fixed port. Provider JSON never leaves this module.

pub mod service;

use serde::{Deserialize, Serialize};
use ts_rs::TS;
use url::Url;

/// Fixed loopback redirect registered in the user's Spotify app settings.
pub const REDIRECT_PORT: u16 = 43821;
pub const REDIRECT_PATH: &str = "/callback";
const AUTH_URL: &str = "https://accounts.spotify.com/authorize";
const TOKEN_URL: &str = "https://accounts.spotify.com/api/token";
const API: &str = "https://api.spotify.com/v1";
/// Read playlists; read devices and start playback (Premium); `user-read-private` tells us
/// whether the account is Premium so we can fall back gracefully.
pub const SCOPES: &str =
    "playlist-read-private playlist-read-collaborative user-read-playback-state user-modify-playback-state user-read-private";
/// Most playlists shown in the notch.
pub const MAX_PINNED: usize = 4;
/// Playlists fetched for the Settings picker.
const MAX_PLAYLISTS: usize = 200;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SpotifyPlaylist {
    pub id: String,
    pub name: String,
    #[ts(optional)]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image_url: Option<String>,
    pub track_count: u32,
    pub owner: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum SpotifyErrorKind {
    AuthExpired,
    Network,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(tag = "state", rename_all = "camelCase")]
#[ts(export)]
pub enum SpotifyStatus {
    NotConfigured,
    Disconnected,
    Connecting,
    Connected,
    Error { reason: SpotifyErrorKind },
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SpotifySnapshot {
    pub status: SpotifyStatus,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub account: Option<String>,
    /// Whether the account can start playback remotely; `None` until known.
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub premium: Option<bool>,
    /// All of the user's playlists (for the Settings picker).
    pub playlists: Vec<SpotifyPlaylist>,
    /// Up to four playlist ids shown in the notch, in order.
    pub pinned: Vec<String>,
    pub mock: bool,
}

/// What happened when the user tapped a playlist.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum PlayOutcome {
    /// Playback started on a Spotify device.
    Playing,
    /// No device was available (or no Premium), so the playlist was opened in the Spotify app.
    OpenedInApp,
}

pub fn authorization_url(client_id: &str, redirect_uri: &str, challenge: &str, state: &str) -> String {
    let mut url = Url::parse(AUTH_URL).expect("static URL");
    url.query_pairs_mut()
        .append_pair("client_id", client_id)
        .append_pair("response_type", "code")
        .append_pair("redirect_uri", redirect_uri)
        .append_pair("code_challenge_method", "S256")
        .append_pair("code_challenge", challenge)
        .append_pair("state", state)
        .append_pair("scope", SCOPES);
    url.into()
}

/// Spotify ids are base-62; reject anything else before it reaches a URL.
pub fn valid_id(id: &str) -> bool {
    !id.is_empty() && id.len() <= 64 && id.chars().all(|c| c.is_ascii_alphanumeric())
}

/// App URI that opens a playlist in the Spotify desktop app.
pub fn playlist_app_uri(id: &str) -> String {
    format!("spotify:playlist:{id}")
}

#[derive(Debug, Deserialize)]
pub struct TokenResponse {
    pub access_token: String,
    pub expires_in: u64,
    #[serde(default)]
    pub refresh_token: Option<String>,
}

#[derive(Debug)]
pub enum ApiError {
    /// Token expired or revoked (401), or `invalid_grant` on refresh.
    Unauthorized,
    /// No active device to play on (404 NO_ACTIVE_DEVICE).
    NoActiveDevice,
    /// Premium required (403).
    Forbidden,
    Network(String),
    Other(String),
}

fn network(e: reqwest::Error) -> ApiError {
    ApiError::Network(e.without_url().to_string())
}

async fn token_request(http: &reqwest::Client, form: &[(&str, &str)]) -> Result<TokenResponse, ApiError> {
    let response = http.post(TOKEN_URL).form(form).send().await.map_err(network)?;
    let status = response.status();
    let body = response.text().await.map_err(network)?;
    if status.is_success() {
        return serde_json::from_str(&body).map_err(|e| ApiError::Other(e.to_string()));
    }
    #[derive(Deserialize)]
    struct TokenError {
        error: String,
    }
    match serde_json::from_str::<TokenError>(&body) {
        Ok(err) if err.error == "invalid_grant" => Err(ApiError::Unauthorized),
        Ok(err) => Err(ApiError::Other(format!("token endpoint: {}", err.error))),
        Err(_) => Err(ApiError::Other(format!("token endpoint returned {status}"))),
    }
}

pub async fn exchange_code(
    http: &reqwest::Client,
    client_id: &str,
    code: &str,
    verifier: &str,
    redirect_uri: &str,
) -> Result<TokenResponse, ApiError> {
    token_request(
        http,
        &[
            ("grant_type", "authorization_code"),
            ("code", code),
            ("redirect_uri", redirect_uri),
            ("client_id", client_id),
            ("code_verifier", verifier),
        ],
    )
    .await
}

pub async fn refresh(
    http: &reqwest::Client,
    client_id: &str,
    refresh_token: &str,
) -> Result<TokenResponse, ApiError> {
    token_request(
        http,
        &[
            ("grant_type", "refresh_token"),
            ("refresh_token", refresh_token),
            ("client_id", client_id),
        ],
    )
    .await
}

fn classify(status: reqwest::StatusCode, body: &str) -> ApiError {
    match status.as_u16() {
        401 => ApiError::Unauthorized,
        403 => ApiError::Forbidden,
        404 if body.contains("NO_ACTIVE_DEVICE")
            || body.to_ascii_lowercase().contains("no active device") =>
        {
            ApiError::NoActiveDevice
        }
        _ => ApiError::Other(format!("Spotify API returned {status}")),
    }
}

async fn get_json(http: &reqwest::Client, token: &str, url: &str) -> Result<serde_json::Value, ApiError> {
    let response = http.get(url).bearer_auth(token).send().await.map_err(network)?;
    let status = response.status();
    let body = response.text().await.map_err(network)?;
    if !status.is_success() {
        return Err(classify(status, &body));
    }
    serde_json::from_str(&body).map_err(|e| ApiError::Other(e.to_string()))
}

#[derive(Debug, Deserialize)]
struct Me {
    #[serde(default)]
    display_name: Option<String>,
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    product: Option<String>,
}

/// Account display name and whether it is Premium.
pub async fn me(http: &reqwest::Client, token: &str) -> Result<(Option<String>, Option<bool>), ApiError> {
    let value = get_json(http, token, &format!("{API}/me")).await?;
    let me: Me = serde_json::from_value(value).map_err(|e| ApiError::Other(e.to_string()))?;
    Ok((me.display_name.or(me.id), me.product.map(|p| p == "premium")))
}

#[derive(Debug, Deserialize)]
struct PlaylistPage {
    #[serde(default)]
    items: Vec<Option<RawPlaylist>>,
    #[serde(default)]
    next: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RawPlaylist {
    id: String,
    #[serde(default)]
    name: String,
    #[serde(default)]
    images: Option<Vec<RawImage>>,
    #[serde(default)]
    tracks: Option<RawTracks>,
    #[serde(default)]
    owner: Option<RawOwner>,
}

#[derive(Debug, Deserialize)]
struct RawImage {
    url: String,
}

#[derive(Debug, Deserialize)]
struct RawTracks {
    #[serde(default)]
    total: u32,
}

#[derive(Debug, Deserialize)]
struct RawOwner {
    #[serde(default)]
    display_name: Option<String>,
}

/// Parse one page of `GET /me/playlists`, dropping null or malformed entries. Treated as
/// untrusted input: only https artwork and well-formed ids survive.
fn parse_playlist_page(value: serde_json::Value) -> Result<(Vec<SpotifyPlaylist>, Option<String>), ApiError> {
    let page: PlaylistPage = serde_json::from_value(value).map_err(|e| ApiError::Other(e.to_string()))?;
    let playlists = page
        .items
        .into_iter()
        .flatten()
        .filter(|p| valid_id(&p.id))
        .map(|p| SpotifyPlaylist {
            image_url: p
                .images
                .unwrap_or_default()
                .into_iter()
                .map(|i| i.url)
                .find(|u| u.starts_with("https://")),
            name: if p.name.trim().is_empty() {
                "Untitled playlist".into()
            } else {
                p.name
            },
            track_count: p.tracks.map(|t| t.total).unwrap_or(0),
            owner: p.owner.and_then(|o| o.display_name).unwrap_or_default(),
            id: p.id,
        })
        .collect();
    let next = page.next.filter(|n| n.starts_with(&format!("{API}/")));
    Ok((playlists, next))
}

pub async fn playlists(http: &reqwest::Client, token: &str) -> Result<Vec<SpotifyPlaylist>, ApiError> {
    let mut all = Vec::new();
    let mut url = Some(format!("{API}/me/playlists?limit=50"));
    while let Some(next) = url.take() {
        let (mut page, next_url) = parse_playlist_page(get_json(http, token, &next).await?)?;
        all.append(&mut page);
        if all.len() < MAX_PLAYLISTS {
            url = next_url;
        }
    }
    all.truncate(MAX_PLAYLISTS);
    Ok(all)
}

#[derive(Debug, Deserialize)]
struct Devices {
    #[serde(default)]
    devices: Vec<RawDevice>,
}

#[derive(Debug, Deserialize)]
struct RawDevice {
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    is_active: bool,
    #[serde(default)]
    is_restricted: bool,
    #[serde(default, rename = "type")]
    kind: String,
}

/// Best device to start playback on: the active one, else this computer, else any.
fn pick_device(devices: Devices) -> Option<String> {
    let usable: Vec<RawDevice> = devices
        .devices
        .into_iter()
        .filter(|d| !d.is_restricted && d.id.is_some())
        .collect();
    usable
        .iter()
        .find(|d| d.is_active)
        .or_else(|| usable.iter().find(|d| d.kind.eq_ignore_ascii_case("computer")))
        .or_else(|| usable.first())
        .and_then(|d| d.id.clone())
}

async fn put_play(
    http: &reqwest::Client,
    token: &str,
    playlist_id: &str,
    device: Option<&str>,
) -> Result<(), ApiError> {
    let mut url = Url::parse(&format!("{API}/me/player/play")).expect("static URL");
    if let Some(device) = device {
        url.query_pairs_mut().append_pair("device_id", device);
    }
    let response = http
        .put(url)
        .bearer_auth(token)
        .json(&serde_json::json!({ "context_uri": format!("spotify:playlist:{playlist_id}") }))
        .send()
        .await
        .map_err(network)?;
    let status = response.status();
    if status.is_success() {
        return Ok(());
    }
    let body = response.text().await.unwrap_or_default();
    Err(classify(status, &body))
}

/// Start a playlist on the active device, or on this computer's Spotify if nothing is active.
pub async fn play_playlist(http: &reqwest::Client, token: &str, playlist_id: &str) -> Result<(), ApiError> {
    match put_play(http, token, playlist_id, None).await {
        Err(ApiError::NoActiveDevice) => {
            let value = get_json(http, token, &format!("{API}/me/player/devices")).await?;
            let devices: Devices =
                serde_json::from_value(value).map_err(|e| ApiError::Other(e.to_string()))?;
            let device = pick_device(devices).ok_or(ApiError::NoActiveDevice)?;
            put_play(http, token, playlist_id, Some(&device)).await
        }
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn authorization_url_uses_pkce_and_the_registered_redirect() {
        let url = Url::parse(&authorization_url(
            "cid",
            "http://127.0.0.1:43821/callback",
            "chal",
            "st",
        ))
        .unwrap();
        let q: std::collections::HashMap<_, _> = url.query_pairs().into_owned().collect();
        assert_eq!(q["code_challenge_method"], "S256");
        assert_eq!(q["redirect_uri"], "http://127.0.0.1:43821/callback");
        assert!(q["scope"].contains("user-modify-playback-state"));
        assert!(!q.contains_key("client_secret"));
    }

    #[test]
    fn parses_playlists_defensively() {
        let json = serde_json::json!({
            "items": [
                { "id": "37i9dQZF1DXcBWIGoYBM5M", "name": "Today's Top Hits",
                  "images": [{ "url": "https://i.scdn.co/image/abc" }],
                  "tracks": { "total": 50 }, "owner": { "display_name": "Spotify" } },
                null,
                { "id": "bad/id", "name": "Nope" },
                { "id": "abc123", "name": " ", "images": [{ "url": "http://insecure" }], "tracks": null }
            ],
            "next": "https://api.spotify.com/v1/me/playlists?offset=50&limit=50"
        });
        let (playlists, next) = parse_playlist_page(json).unwrap();
        assert_eq!(playlists.len(), 2);
        assert_eq!(
            playlists[0].image_url.as_deref(),
            Some("https://i.scdn.co/image/abc")
        );
        assert_eq!(playlists[0].track_count, 50);
        assert_eq!(playlists[1].name, "Untitled playlist");
        assert_eq!(playlists[1].image_url, None);
        assert!(next.is_some());
    }

    #[test]
    fn ignores_pagination_links_off_the_api_host() {
        let json = serde_json::json!({ "items": [], "next": "https://evil.example/steal" });
        assert_eq!(parse_playlist_page(json).unwrap().1, None);
    }

    #[test]
    fn picks_the_active_device_then_this_computer() {
        let devices = |json: serde_json::Value| serde_json::from_value::<Devices>(json).unwrap();
        let d = devices(serde_json::json!({ "devices": [
            { "id": "phone", "type": "Smartphone", "is_active": false },
            { "id": "pc", "type": "Computer", "is_active": false },
            { "id": "tv", "type": "TV", "is_active": true }
        ]}));
        assert_eq!(pick_device(d).as_deref(), Some("tv"));
        let d = devices(serde_json::json!({ "devices": [
            { "id": "phone", "type": "Smartphone" }, { "id": "pc", "type": "Computer" },
            { "id": "locked", "type": "Computer", "is_restricted": true }
        ]}));
        assert_eq!(pick_device(d).as_deref(), Some("pc"));
        assert_eq!(pick_device(devices(serde_json::json!({ "devices": [] }))), None);
    }

    #[test]
    fn classifies_player_errors() {
        use reqwest::StatusCode as S;
        assert!(matches!(
            classify(S::NOT_FOUND, r#"{"error":{"reason":"NO_ACTIVE_DEVICE"}}"#),
            ApiError::NoActiveDevice
        ));
        assert!(matches!(classify(S::FORBIDDEN, ""), ApiError::Forbidden));
        assert!(matches!(classify(S::UNAUTHORIZED, ""), ApiError::Unauthorized));
    }

    #[test]
    fn validates_ids() {
        assert!(valid_id("37i9dQZF1DXcBWIGoYBM5M"));
        assert!(!valid_id("../x"));
        assert!(!valid_id(""));
    }
}
