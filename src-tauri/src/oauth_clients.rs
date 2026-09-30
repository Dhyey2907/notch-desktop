//! Notch's own registered OAuth clients, baked in at build time from `oauth-clients.json`
//! (see `build.rs`). With them, Settings offers one-click "Continue with …" buttons; without,
//! users paste their own client IDs as before.
//!
//! These are *public* installed-app clients: Google documents that a desktop app's client
//! secret can't be kept confidential, and PKCE / the device flow protect the sign-in itself.

use serde::Serialize;
use ts_rs::TS;

fn non_empty(value: Option<&'static str>) -> Option<&'static str> {
    value.filter(|v| !v.is_empty())
}

/// Built-in Google client (ID, secret).
pub fn google() -> Option<(&'static str, &'static str)> {
    Some((
        non_empty(option_env!("NOTCH_GOOGLE_CLIENT_ID"))?,
        non_empty(option_env!("NOTCH_GOOGLE_CLIENT_SECRET"))?,
    ))
}

pub fn spotify() -> Option<&'static str> {
    non_empty(option_env!("NOTCH_SPOTIFY_CLIENT_ID"))
}

pub fn github() -> Option<&'static str> {
    non_empty(option_env!("NOTCH_GITHUB_CLIENT_ID"))
}

/// Which one-click sign-ins this build offers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SignInOptions {
    pub google: bool,
    pub spotify: bool,
    pub github: bool,
}

pub fn options() -> SignInOptions {
    SignInOptions {
        google: google().is_some(),
        spotify: spotify().is_some(),
        github: github().is_some(),
    }
}
