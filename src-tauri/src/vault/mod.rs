//! Vault: logins kept as notes in the user's Obsidian vault (passwords encrypted with a key
//! that lives only in Windows Credential Manager), and 2FA codes. Copying a password keeps it
//! out of clipboard history and clears it after a short while.

pub mod entry;
#[cfg(windows)]
mod hello;
pub mod service;
pub mod totp;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Folder inside the Obsidian vault that holds the logins.
pub const SUBFOLDER: &str = "Notch Vault";

/// A login as the UI sees it — never with its password.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct VaultEntry {
    /// The note's file name (stable while it isn't renamed).
    pub id: String,
    pub title: String,
    pub username: String,
    pub url: String,
    pub tags: Vec<String>,
    pub notes: String,
    pub has_password: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct VaultState {
    /// The Obsidian vault folder, once chosen.
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub folder: Option<String>,
    pub entries: Vec<VaultEntry>,
    /// Windows Hello is set up on this PC.
    pub hello_available: bool,
}

/// A login being added or edited. `password: None` keeps the saved one.
#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct VaultInput {
    #[ts(optional)]
    pub id: Option<String>,
    pub title: String,
    pub username: String,
    pub url: String,
    #[ts(optional)]
    pub password: Option<String>,
    pub tags: Vec<String>,
    pub notes: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum VaultField {
    Username,
    Password,
}

/// A 2FA code right now.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct TotpCode {
    pub id: String,
    pub issuer: String,
    pub label: String,
    pub code: String,
    /// Seconds until the code changes.
    pub remaining: u32,
    pub period: u32,
}

/// A strong random password: letters, digits and a few symbols, no look-alikes.
pub fn generate_password(length: usize) -> String {
    const CHARS: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz23456789!@#$%&*-_+=?";
    let length = length.clamp(12, 64);
    let mut bytes = vec![0u8; length * 2];
    loop {
        rand::fill(&mut bytes[..]);
        // Rejection sampling keeps every character equally likely.
        let limit = 256 - (256 % CHARS.len());
        let password: String = bytes
            .iter()
            .filter(|&&b| usize::from(b) < limit)
            .take(length)
            .map(|&b| CHARS[usize::from(b) % CHARS.len()] as char)
            .collect();
        let has = |f: fn(&char) -> bool| password.chars().any(|c| f(&c));
        if password.len() == length
            && has(char::is_ascii_lowercase)
            && has(char::is_ascii_uppercase)
            && has(char::is_ascii_digit)
            && has(char::is_ascii_punctuation)
        {
            return password;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generates_strong_passwords() {
        let a = generate_password(20);
        let b = generate_password(20);
        assert_eq!(a.len(), 20);
        assert_ne!(a, b);
        assert!(a.chars().any(|c| c.is_ascii_digit()));
        assert!(!a.contains(['0', 'O', 'l', '1', 'I']));
        assert_eq!(generate_password(3).len(), 12);
    }
}
