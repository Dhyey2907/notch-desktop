//! Google OAuth 2.0 for installed apps: authorization code + PKCE with a loopback redirect
//! (https://developers.google.com/identity/protocols/oauth2/native-app). The browser does
//! the sign-in; this app never sees the user's password. The refresh token goes straight
//! to Windows Credential Manager.

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use serde::Deserialize;
use url::Url;

const AUTH_URL: &str = "https://accounts.google.com/o/oauth2/v2/auth";
const TOKEN_URL: &str = "https://oauth2.googleapis.com/token";
const REVOKE_URL: &str = "https://oauth2.googleapis.com/revoke";
/// Least privilege: read-only calendar access, plus the email to label the account.
pub const SCOPES: &str = "https://www.googleapis.com/auth/calendar.readonly openid email";
/// Gmail glance: read the inbox, archive and mark read (not send, not delete).
pub const GMAIL_SCOPE: &str = "https://www.googleapis.com/auth/gmail.modify";
/// Assignment deadlines: your classes and your own course work, read-only.
pub const CLASSROOM_SCOPES: &str = "https://www.googleapis.com/auth/classroom.courses.readonly https://www.googleapis.com/auth/classroom.coursework.me.readonly";

/// Calendar's scopes plus the extras the user turned on.
pub fn scopes_for(gmail: bool, classroom: bool) -> String {
    let mut scopes = SCOPES.to_owned();
    if gmail {
        scopes = format!("{scopes} {GMAIL_SCOPE}");
    }
    if classroom {
        scopes = format!("{scopes} {CLASSROOM_SCOPES}");
    }
    scopes
}

pub fn authorization_url(
    client_id: &str,
    redirect_uri: &str,
    challenge: &str,
    state: &str,
    scopes: &str,
) -> String {
    let mut url = Url::parse(AUTH_URL).expect("static URL");
    url.query_pairs_mut()
        .append_pair("client_id", client_id)
        .append_pair("redirect_uri", redirect_uri)
        .append_pair("response_type", "code")
        .append_pair("scope", scopes)
        .append_pair("code_challenge", challenge)
        .append_pair("code_challenge_method", "S256")
        .append_pair("state", state)
        // Offline access + consent so Google always returns a refresh token.
        .append_pair("access_type", "offline")
        .append_pair("prompt", "consent");
    url.into()
}

#[derive(Debug, Deserialize)]
pub struct TokenResponse {
    pub access_token: String,
    pub expires_in: u64,
    #[serde(default)]
    pub refresh_token: Option<String>,
    #[serde(default)]
    pub id_token: Option<String>,
    /// What the user actually allowed (they can untick extras on Google's screen).
    #[serde(default)]
    pub scope: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TokenError {
    error: String,
}

/// Why a token request failed; `InvalidGrant` means the user must sign in again.
#[derive(Debug)]
pub enum TokenFailure {
    InvalidGrant,
    Network(String),
    Other(String),
}

async fn token_request(http: &reqwest::Client, form: &[(&str, &str)]) -> Result<TokenResponse, TokenFailure> {
    let response = http
        .post(TOKEN_URL)
        .form(form)
        .send()
        .await
        .map_err(|e| TokenFailure::Network(e.without_url().to_string()))?;
    let status = response.status();
    let body = response
        .text()
        .await
        .map_err(|e| TokenFailure::Network(e.without_url().to_string()))?;
    if status.is_success() {
        return serde_json::from_str(&body).map_err(|e| TokenFailure::Other(e.to_string()));
    }
    match serde_json::from_str::<TokenError>(&body) {
        Ok(err) if err.error == "invalid_grant" => Err(TokenFailure::InvalidGrant),
        // Only the error code is kept; Google's descriptions can echo request details.
        Ok(err) => Err(TokenFailure::Other(format!("token endpoint: {}", err.error))),
        Err(_) => Err(TokenFailure::Other(format!("token endpoint returned {status}"))),
    }
}

pub async fn exchange_code(
    http: &reqwest::Client,
    client_id: &str,
    client_secret: &str,
    code: &str,
    verifier: &str,
    redirect_uri: &str,
) -> Result<TokenResponse, TokenFailure> {
    token_request(
        http,
        &[
            ("client_id", client_id),
            ("client_secret", client_secret),
            ("code", code),
            ("code_verifier", verifier),
            ("redirect_uri", redirect_uri),
            ("grant_type", "authorization_code"),
        ],
    )
    .await
}

pub async fn refresh(
    http: &reqwest::Client,
    client_id: &str,
    client_secret: &str,
    refresh_token: &str,
) -> Result<TokenResponse, TokenFailure> {
    token_request(
        http,
        &[
            ("client_id", client_id),
            ("client_secret", client_secret),
            ("refresh_token", refresh_token),
            ("grant_type", "refresh_token"),
        ],
    )
    .await
}

/// Best-effort revocation on disconnect; local credentials are removed regardless.
pub async fn revoke(http: &reqwest::Client, token: &str) {
    if let Err(error) = http.post(REVOKE_URL).form(&[("token", token)]).send().await {
        tracing::debug!(error = %error.without_url(), "token revocation failed");
    }
}

/// The `email` claim of an ID token, used only as a display label for the connected
/// account. It came straight from Google's token endpoint over TLS, so it isn't verified.
pub fn email_from_id_token(id_token: &str) -> Option<String> {
    let payload = id_token.split('.').nth(1)?;
    let bytes = URL_SAFE_NO_PAD.decode(payload.trim_end_matches('=')).ok()?;
    let claims: serde_json::Value = serde_json::from_slice(&bytes).ok()?;
    claims.get("email")?.as_str().map(str::to_owned)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn authorization_url_requests_offline_read_only_access_with_pkce() {
        let url = Url::parse(&authorization_url(
            "cid",
            "http://127.0.0.1:5000",
            "chal",
            "st",
            SCOPES,
        ))
        .unwrap();
        let q: std::collections::HashMap<_, _> = url.query_pairs().into_owned().collect();
        assert_eq!(q["scope"], SCOPES);
        assert_eq!(q["code_challenge_method"], "S256");
        assert_eq!(q["access_type"], "offline");
        assert_eq!(q["redirect_uri"], "http://127.0.0.1:5000");
    }

    #[test]
    fn reads_the_email_claim_for_display() {
        let payload = URL_SAFE_NO_PAD.encode(br#"{"email":"me@example.com","sub":"1"}"#);
        assert_eq!(
            email_from_id_token(&format!("h.{payload}.sig")).as_deref(),
            Some("me@example.com")
        );
        assert_eq!(email_from_id_token("not-a-jwt"), None);
    }
}
