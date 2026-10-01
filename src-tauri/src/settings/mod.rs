//! Typed, non-secret user preferences (architecture.md §9). Stored as one JSON document in
//! the `settings` table; unknown or missing fields fall back to defaults, so adding a field
//! never needs a migration. Secrets never live here — they go to Windows Credential Manager.

use std::collections::BTreeMap;

use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::error::{AppError, AppResult};

const SETTINGS_KEY: &str = "app";

/// A module that fails to initialize this many times in a row is disabled automatically.
pub const MAX_INIT_FAILURES: u32 = 3;
const MAX_MODULES: usize = 100;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export)]
pub enum AnimationPreference {
    /// Follow the Windows "animation effects" setting.
    #[default]
    System,
    /// Always use reduced motion.
    Reduced,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export)]
pub struct ModuleSettings {
    pub enabled: bool,
    /// Whether the module may interrupt with non-critical activities.
    pub may_peek: bool,
    /// Consecutive failed initializations; reset on success.
    pub init_failures: u32,
    /// Set when the app disabled the module after repeated failures, so Settings can say why.
    pub auto_disabled: bool,
}

impl Default for ModuleSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            may_peek: true,
            init_failures: 0,
            auto_disabled: false,
        }
    }
}

/// Calendar preferences. The OAuth client ID is not secret (it ships inside installed apps);
/// the client secret and tokens live in Credential Manager.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export)]
pub struct CalendarSettings {
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_id: Option<String>,
    /// Display label for the connected Google account.
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub account_email: Option<String>,
    /// Minutes before a meeting to show the countdown; 0 turns reminders off.
    pub reminder_minutes: u32,
}

impl Default for CalendarSettings {
    fn default() -> Self {
        Self {
            client_id: None,
            account_email: None,
            reminder_minutes: 5,
        }
    }
}

/// Spotify preferences. The client ID is public (PKCE, no secret); the refresh token lives in
/// Credential Manager.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export)]
pub struct SpotifySettings {
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_id: Option<String>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub account_name: Option<String>,
    /// Up to four playlist ids shown in the Media module, in order.
    pub pinned_playlists: Vec<String>,
}

/// Which screen(s) show the notch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum DisplayMode {
    /// The Windows main display.
    #[default]
    Primary,
    /// One chosen screen (falls back to the main display while it's disconnected).
    One,
    /// Every connected screen.
    All,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export)]
pub struct DisplaySettings {
    pub mode: DisplayMode,
    /// Windows device name of the chosen screen (e.g. `\\.\DISPLAY2`) for `One`.
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub monitor: Option<String>,
}

/// Volume, brightness and Bluetooth readouts in the notch (all shown unless turned off).
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export)]
pub struct HudSettings {
    pub hide_volume: bool,
    pub hide_brightness: bool,
    pub hide_bluetooth: bool,
}

/// Clipboard history: pause it, or never keep copies from some apps (by exe name).
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export)]
pub struct ClipboardSettings {
    pub paused: bool,
    /// e.g. `["slack.exe"]`. Password managers are always skipped.
    pub blocked_apps: Vec<String>,
}

/// Repositories the Git module knows about (most recent first) and the one shown.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export)]
pub struct GitSettings {
    pub repos: Vec<String>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active: Option<String>,
    /// GitHub repository (`owner/name`) picked while signed in with GitHub.
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub github_repo: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export)]
pub struct Settings {
    /// The first-run prompt (e.g. start with Windows) has been answered.
    pub first_run_completed: bool,
    pub animation: AnimationPreference,
    /// Per-module preferences keyed by module id. Missing modules use defaults.
    pub modules: BTreeMap<String, ModuleSettings>,
    pub calendar: CalendarSettings,
    pub spotify: SpotifySettings,
    pub git: GitSettings,
    pub display: DisplaySettings,
    pub clipboard: ClipboardSettings,
    pub hud: HudSettings,
}

impl Settings {
    /// Reject input that could only come from a bug or tampering.
    pub fn validate(&self) -> AppResult<()> {
        if self.modules.len() > MAX_MODULES {
            return Err(AppError::InvalidInput("too many module entries".into()));
        }
        if self.spotify.pinned_playlists.len() > crate::spotify::MAX_PINNED
            || !self
                .spotify
                .pinned_playlists
                .iter()
                .all(|id| crate::spotify::valid_id(id))
        {
            return Err(AppError::InvalidInput("invalid pinned playlists".into()));
        }
        if self.git.repos.len() > crate::git::service::MAX_RECENT_REPOS
            || self
                .git
                .repos
                .iter()
                .chain(self.git.active.iter())
                .any(|p| p.len() > 400)
        {
            return Err(AppError::InvalidInput("invalid repository list".into()));
        }
        if self
            .display
            .monitor
            .as_ref()
            .is_some_and(|m| m.is_empty() || m.len() > 64)
        {
            return Err(AppError::InvalidInput("invalid screen name".into()));
        }
        if self
            .git
            .github_repo
            .as_deref()
            .is_some_and(|r| !crate::github::valid_name_with_owner(r))
        {
            return Err(AppError::InvalidInput("invalid GitHub repository".into()));
        }
        if self.clipboard.blocked_apps.len() > 50
            || self
                .clipboard
                .blocked_apps
                .iter()
                .any(|a| a.trim().is_empty() || a.len() > 100)
        {
            return Err(AppError::InvalidInput("invalid clipboard block list".into()));
        }
        if self.calendar.reminder_minutes > 120 {
            return Err(AppError::InvalidInput("reminder lead is too long".into()));
        }
        for id in self.modules.keys() {
            let valid = !id.is_empty()
                && id.len() <= 64
                && id
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
            if !valid {
                return Err(AppError::InvalidInput(format!("invalid module id: {id:?}")));
            }
        }
        Ok(())
    }

    /// Record the outcome of a module's initialization, auto-disabling it after repeated
    /// consecutive failures (architecture.md §10: recoverable path for failing modules).
    pub fn record_module_init(&mut self, module_id: &str, ok: bool) {
        let entry = self.modules.entry(module_id.to_owned()).or_default();
        if ok {
            entry.init_failures = 0;
            return;
        }
        entry.init_failures += 1;
        if entry.init_failures >= MAX_INIT_FAILURES && entry.enabled {
            entry.enabled = false;
            entry.auto_disabled = true;
            tracing::warn!(module = module_id, "module disabled after repeated init failures");
        }
    }
}

pub fn load(conn: &Connection) -> AppResult<Settings> {
    let raw: Option<String> = conn
        .query_row(
            "SELECT value FROM settings WHERE key = ?1",
            [SETTINGS_KEY],
            |row| row.get(0),
        )
        .optional()?;
    let Some(raw) = raw else {
        return Ok(Settings::default());
    };
    match serde_json::from_str(&raw) {
        Ok(settings) => Ok(settings),
        Err(error) => {
            // Don't crash the app over a corrupt row; start from defaults and say so.
            tracing::warn!(%error, "stored settings are unreadable; using defaults");
            Ok(Settings::default())
        }
    }
}

pub fn save(conn: &Connection, settings: &Settings) -> AppResult<()> {
    let json = serde_json::to_string(settings).map_err(|e| AppError::InvalidInput(e.to_string()))?;
    conn.execute(
        "INSERT INTO settings (key, value, updated_at) VALUES (?1, ?2, ?3)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at",
        params![SETTINGS_KEY, json, chrono::Utc::now().to_rfc3339()],
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Db;

    fn db() -> Db {
        Db::open(std::path::Path::new(":memory:")).unwrap()
    }

    #[test]
    fn defaults_when_nothing_is_stored() {
        let db = db();
        assert_eq!(db.with(load).unwrap(), Settings::default());
    }

    #[test]
    fn round_trips_through_the_database() {
        let db = db();
        let mut settings = Settings {
            animation: AnimationPreference::Reduced,
            ..Default::default()
        };
        settings.modules.insert(
            "media".into(),
            ModuleSettings {
                may_peek: false,
                ..Default::default()
            },
        );
        db.with(|c| save(c, &settings)).unwrap();
        assert_eq!(db.with(load).unwrap(), settings);
    }

    #[test]
    fn fills_missing_fields_with_defaults() {
        let parsed: Settings = serde_json::from_str(r#"{"modules":{"git":{"mayPeek":false}}}"#).unwrap();
        let git = &parsed.modules["git"];
        assert!(git.enabled);
        assert!(!git.may_peek);
        assert_eq!(parsed.animation, AnimationPreference::System);
    }

    #[test]
    fn falls_back_to_defaults_on_corrupt_json() {
        let db = db();
        db.with(|c| {
            c.execute(
                "INSERT INTO settings (key, value, updated_at) VALUES ('app', '{nope', 'now')",
                [],
            )?;
            Ok(())
        })
        .unwrap();
        assert_eq!(db.with(load).unwrap(), Settings::default());
    }

    #[test]
    fn auto_disables_after_repeated_failures_and_resets_on_success() {
        let mut settings = Settings::default();
        settings.record_module_init("media", false);
        settings.record_module_init("media", true);
        assert_eq!(settings.modules["media"].init_failures, 0);

        for _ in 0..MAX_INIT_FAILURES {
            settings.record_module_init("media", false);
        }
        let media = &settings.modules["media"];
        assert!(!media.enabled);
        assert!(media.auto_disabled);
    }

    #[test]
    fn rejects_bad_module_ids() {
        let mut settings = Settings::default();
        settings
            .modules
            .insert("../evil".into(), ModuleSettings::default());
        assert!(settings.validate().is_err());
    }
}
