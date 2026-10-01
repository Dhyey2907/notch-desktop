//! Secure credential storage (architecture.md §3 `SecureCredentialPort`): secrets live in
//! Windows Credential Manager, never in SQLite, settings, or logs.

use crate::error::{AppError, AppResult};

const SERVICE: &str = "com.notchdesktop.app";

/// Names of stored secrets. Keeping them in one enum makes every secret the app holds
/// easy to audit (and to clear on disconnect).
#[derive(Debug, Clone, Copy)]
pub enum Credential {
    GoogleClientSecret,
    GoogleRefreshToken,
    SpotifyRefreshToken,
    /// Shared secret Claude Code presents to Notch's local hook server.
    ClaudeHookToken,
    /// "Continue with GitHub" access token (pull requests).
    GithubToken,
    /// Key that encrypts passwords in the Obsidian vault (base64, 32 bytes).
    VaultKey,
    #[cfg(test)]
    Test,
}

impl Credential {
    fn key(self) -> &'static str {
        match self {
            Credential::GoogleClientSecret => "google-calendar/client-secret",
            Credential::GoogleRefreshToken => "google-calendar/refresh-token",
            Credential::SpotifyRefreshToken => "spotify/refresh-token",
            Credential::ClaudeHookToken => "claude-code/hook-token",
            Credential::GithubToken => "github/token",
            Credential::VaultKey => "vault/key",
            #[cfg(test)]
            Credential::Test => "test/round-trip",
        }
    }
}

fn entry(secret: Credential) -> AppResult<keyring::Entry> {
    keyring::Entry::new(SERVICE, secret.key()).map_err(|e| AppError::Platform(e.to_string()))
}

/// A 2FA account's secret (one per account, keyed by its id).
fn totp_entry(id: &str) -> AppResult<keyring::Entry> {
    if id.is_empty() || id.len() > 64 || !id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
        return Err(AppError::InvalidInput("invalid account id".into()));
    }
    keyring::Entry::new(SERVICE, &format!("totp/{id}")).map_err(|e| AppError::Platform(e.to_string()))
}

pub fn totp_get(id: &str) -> AppResult<Option<String>> {
    match totp_entry(id)?.get_password() {
        Ok(value) => Ok(Some(value)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(error) => Err(AppError::Platform(error.to_string())),
    }
}

pub fn totp_set(id: &str, secret: &str) -> AppResult<()> {
    totp_entry(id)?
        .set_password(secret)
        .map_err(|e| AppError::Platform(e.to_string()))
}

pub fn totp_delete(id: &str) -> AppResult<()> {
    match totp_entry(id)?.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(error) => Err(AppError::Platform(error.to_string())),
    }
}

pub fn get(secret: Credential) -> AppResult<Option<String>> {
    match entry(secret)?.get_password() {
        Ok(value) => Ok(Some(value)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(error) => Err(AppError::Platform(error.to_string())),
    }
}

pub fn set(secret: Credential, value: &str) -> AppResult<()> {
    entry(secret)?
        .set_password(value)
        .map_err(|e| AppError::Platform(e.to_string()))
}

pub fn delete(secret: Credential) -> AppResult<()> {
    match entry(secret)?.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(error) => Err(AppError::Platform(error.to_string())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Touches the real Windows Credential Manager, so it only runs on request:
    /// `cargo test secrets -- --ignored`.
    #[test]
    #[ignore]
    fn round_trips_through_credential_manager() {
        delete(Credential::Test).unwrap();
        assert_eq!(get(Credential::Test).unwrap(), None);
        set(Credential::Test, "s3cret").unwrap();
        assert_eq!(get(Credential::Test).unwrap().as_deref(), Some("s3cret"));
        delete(Credential::Test).unwrap();
        assert_eq!(get(Credential::Test).unwrap(), None);
        delete(Credential::Test).unwrap(); // deleting a missing entry is fine
    }
}
