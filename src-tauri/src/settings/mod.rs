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
    /// Also ask Google for Gmail (glance, archive, mark read).
    pub gmail: bool,
    /// Also ask Google for Classroom (assignment deadlines).
    pub classroom: bool,
    /// Also ask Google to add events and answer invites (Calendar actions).
    pub edit: bool,
    /// Scopes Google granted at the last sign-in.
    pub granted_scopes: Vec<String>,
}

impl Default for CalendarSettings {
    fn default() -> Self {
        Self {
            client_id: None,
            account_email: None,
            reminder_minutes: 5,
            gmail: false,
            classroom: false,
            edit: false,
            granted_scopes: Vec::new(),
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

/// A three-step size (notch size, text size).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum Scale {
    Small,
    #[default]
    Normal,
    Large,
}

/// How the notch looks when nothing is happening.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum BubbleStyle {
    #[default]
    Pill,
    Dot,
    /// Invisible until the pointer reaches it.
    Hidden,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum Theme {
    #[default]
    Black,
    Graphite,
    /// Translucent.
    Glass,
}

/// Look of the notch. Numbers are percentages unless noted.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export)]
pub struct AppearanceSettings {
    pub size: Scale,
    pub text_size: Scale,
    pub bubble: BubbleStyle,
    /// Show the time in the resting bubble.
    pub bubble_clock: bool,
    pub theme: Theme,
    /// 40–100.
    pub opacity: u8,
    /// 0 (square-ish) – 100 (fully round).
    pub roundness: u8,
    /// `#rrggbb`, or empty for the default white.
    pub accent: String,
    /// Tint highlights with the playing album's colors instead.
    pub accent_from_music: bool,
    /// Logical px left (−) or right (+) of center.
    pub offset_x: i32,
    /// Logical px down from the top edge.
    pub offset_y: u32,
}

impl Default for AppearanceSettings {
    fn default() -> Self {
        Self {
            size: Scale::Normal,
            text_size: Scale::Normal,
            bubble: BubbleStyle::Pill,
            bubble_clock: false,
            theme: Theme::Black,
            opacity: 100,
            roundness: 100,
            accent: String::new(),
            accent_from_music: false,
            offset_x: 0,
            offset_y: 0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum Speed {
    Fast,
    #[default]
    Normal,
    Slow,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum AlertSound {
    #[default]
    Off,
    Soft,
    Chime,
}

/// What the notch does over a fullscreen app.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum FullscreenBehavior {
    /// Hidden, but notifications still pop in.
    #[default]
    Alerts,
    /// Hidden; only must-answer cards (approvals, timers) pop in.
    Hide,
    /// Always shown.
    Show,
}

/// Quiet hours: cards wait (like focus) between two times of day, in minutes after midnight.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export)]
pub struct QuietHours {
    pub enabled: bool,
    pub start_min: u32,
    pub end_min: u32,
}

impl Default for QuietHours {
    fn default() -> Self {
        Self {
            enabled: false,
            start_min: 22 * 60,
            end_min: 7 * 60,
        }
    }
}

/// How the notch reacts to the pointer and how long cards stay.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export)]
pub struct BehaviorSettings {
    /// Don't show the small preview when the pointer rests on the bubble.
    pub no_hover_peek: bool,
    pub hover_speed: Speed,
    /// Hovering opens the full notch (instead of clicking).
    pub open_on_hover: bool,
    /// Close the open notch this many seconds after the pointer leaves (0 = only on click away).
    pub auto_close_secs: u32,
    /// How long notifications stay: `Fast` = short, `Slow` = long.
    pub card_time: Speed,
    pub quiet: QuietHours,
    pub sound: AlertSound,
    /// 0–1.
    pub sound_volume: f32,
    pub fullscreen: FullscreenBehavior,
}

impl Default for BehaviorSettings {
    fn default() -> Self {
        Self {
            no_hover_peek: false,
            hover_speed: Speed::Normal,
            open_on_hover: false,
            auto_close_secs: 0,
            card_time: Speed::Normal,
            quiet: QuietHours::default(),
            sound: AlertSound::Off,
            sound_volume: 0.6,
            fullscreen: FullscreenBehavior::Alerts,
        }
    }
}

/// Global shortcuts as text, e.g. `Ctrl+Alt+N`; empty turns one off.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export)]
pub struct ShortcutSettings {
    pub open: String,
    pub ask: String,
    pub clipboard: String,
    /// Grab text from the screen.
    pub grab: String,
    /// Pin the window in front on top (or unpin it).
    pub pin: String,
    /// Quick capture: type a task, reminder, note, event or link from anywhere.
    pub capture: String,
}

impl Default for ShortcutSettings {
    fn default() -> Self {
        Self {
            open: "Ctrl+Alt+N".into(),
            ask: "Ctrl+Alt+A".into(),
            clipboard: "Ctrl+Alt+V".into(),
            grab: "Ctrl+Alt+G".into(),
            pin: "Ctrl+Alt+P".into(),
            capture: "Ctrl+Alt+Space".into(),
        }
    }
}

/// Touchpad swipes on the notch.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export)]
pub struct GestureSettings {
    pub disabled: bool,
    /// `Fast` = a light flick is enough.
    pub sensitivity: Speed,
    /// Left/right on the bubble doesn't change the song.
    pub no_track_swipe: bool,
}

/// The module dock: order, hidden modules, and which one opens first.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export)]
pub struct DockSettings {
    pub order: Vec<String>,
    pub hidden: Vec<String>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub first: Option<String>,
}

/// Home dashboard tiles and units.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export)]
pub struct HomeSettings {
    /// Tile ids: `system`, `controls`, `screen`, `weather`, `desktops`.
    pub hidden_tiles: Vec<String>,
    pub fahrenheit: bool,
    pub clock24h: bool,
}

/// Game mode: which fullscreen apps are games (by exe name) and which aren't.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export)]
pub struct GameSettings {
    pub games: Vec<String>,
    pub not_games: Vec<String>,
    /// Game mode off entirely.
    pub disabled: bool,
}

/// Ambient sound mixer (0–1 per sound) and the bubble's pet.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export)]
pub struct FocusSettings {
    pub rain: f32,
    pub wind: f32,
    pub ocean: f32,
    pub fire: f32,
    pub brown: f32,
    pub pink: f32,
    /// Overall volume (0 = unset, treated as 0.6).
    pub master: f32,
    /// Sounds are playing.
    pub sounds_on: bool,
    /// Which screen's notch plays them (the one the mixer was last used on — it has the
    /// click WebView2 needs before it may play audio).
    pub sound_owner: String,
    /// Sounds stop at this time (epoch ms), if a sleep timer is set.
    #[ts(optional, type = "number")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sleep_until_ms: Option<i64>,
    pub hide_pet: bool,
}

/// Music-reactive notch and lyrics (both on unless turned off).
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export)]
pub struct MusicSettings {
    /// Don't pulse or tint the notch with the music.
    pub hide_reactive: bool,
    /// Don't look up lyrics (that sends the song's title and artist to LRCLIB).
    pub hide_lyrics: bool,
    /// Keep the glow but no colored outline around the notch.
    pub hide_border: bool,
}

/// Which new GitHub notifications pop up in the notch (all are listed in the Git inbox).
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export)]
pub struct GithubAlertSettings {
    pub hide_reviews: bool,
    pub hide_ci_failures: bool,
    /// Successful runs are quiet unless asked for.
    pub show_ci_passes: bool,
    /// Mentions and assignments.
    pub hide_mentions: bool,
}

/// Volume, brightness and Bluetooth readouts in the notch (all shown unless turned off).
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export)]
pub struct HudSettings {
    pub hide_volume: bool,
    pub hide_brightness: bool,
    pub hide_bluetooth: bool,
    /// Caps Lock and Num Lock.
    pub hide_caps: bool,
    /// Keyboard language switches.
    pub hide_language: bool,
    /// "Internet is down" / "Back online".
    pub hide_internet: bool,
    /// An accessory's battery is low.
    pub hide_device_low: bool,
    /// The charger was plugged in.
    pub hide_charging: bool,
    /// Don't say "unplug" when charged to `charge_alert_at`.
    pub no_charge_alert: bool,
    /// Percent for the unplug reminder (0 = the default, 80).
    pub charge_alert_at: u8,
}

/// Clipboard history: pause it, or never keep copies from some apps (by exe name).
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export)]
pub struct ClipboardSettings {
    pub paused: bool,
    /// e.g. `["slack.exe"]`. Password managers are always skipped.
    pub blocked_apps: Vec<String>,
    /// Don't offer "Remind me" / "Open in Maps" for copied dates and addresses.
    pub no_suggestions: bool,
    /// Leave tracking parameters in copied links.
    pub no_link_cleaning: bool,
    /// Turn text snippets off (the keyboard hook stops).
    pub no_snippets: bool,
}

/// Your day: the workday hours and the morning plan.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export)]
pub struct DaySettings {
    /// Workday start and end, minutes after midnight.
    pub start_min: u32,
    pub end_min: u32,
    /// 1 = Monday … 7 = Sunday.
    pub work_days: Vec<u8>,
    /// Don't make a plan by itself on workday mornings.
    pub no_auto_plan: bool,
    /// Hide the workday progress line under the bubble.
    pub no_workday_bar: bool,
    /// The AI day planner may see today's Google Calendar meetings and Classroom deadlines
    /// (titles and times). Off until the user turns it on (Google's Limited Use policy).
    pub share_google: bool,
}

impl Default for DaySettings {
    fn default() -> Self {
        Self {
            start_min: 9 * 60,
            end_min: 18 * 60,
            work_days: vec![1, 2, 3, 4, 5],
            no_auto_plan: false,
            no_workday_bar: false,
            share_google: false,
        }
    }
}

/// Which AI answers Ask, plans the day and writes commit messages.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export)]
pub struct AiSettings {
    /// `claude`, `codex`, `gemini`, `ollama` or `openai` (any OpenAI-compatible API).
    pub provider: String,
    /// Model per provider (e.g. `{"openai": "gpt-5-mini"}`); missing or empty = the provider's
    /// default (Ollama: the first installed model).
    pub models: BTreeMap<String, String>,
    /// API address for `openai` (e.g. https://api.openai.com/v1) or a non-default Ollama.
    pub base_url: String,
}

impl Default for AiSettings {
    fn default() -> Self {
        Self {
            provider: "claude".into(),
            models: BTreeMap::new(),
            base_url: String::new(),
        }
    }
}

impl AiSettings {
    /// The model chosen for the current provider ("" = its default).
    pub fn model(&self) -> &str {
        self.models.get(&self.provider).map(|m| m.trim()).unwrap_or("")
    }
}

/// The pet: who it is, how it talks, what it reacts to, and what it's done with you.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export)]
pub struct PetSettings {
    /// "" = "Pet".
    pub name: String,
    /// Body color as #rrggbb; "" = the theme's text color.
    pub color: String,
    /// How it talks: cheerful, sarcastic, professional, calm, coach, nerdy or dramatic.
    pub tone: String,
    /// Speech bubbles on at all.
    pub talk: bool,
    /// How often it may speak: quiet, normal or chatty.
    pub chattiness: String,
    /// While you keep it out on the taskbar it says something every this many minutes
    /// (0 = never).
    pub chat_minutes: u32,
    /// What it talks about.
    pub say_meetings: bool,
    pub say_breaks: bool,
    pub say_focus: bool,
    pub say_agents: bool,
    pub say_battery: bool,
    pub say_weather: bool,
    pub say_late: bool,
    pub say_greeting: bool,
    /// What it reacts to.
    pub react_music: bool,
    pub react_video: bool,
    pub react_coding: bool,
    pub react_weather: bool,
    pub react_battery: bool,
    pub react_day: bool,
    /// Clicking it pats it.
    pub petting: bool,
    /// When it goes back from its walk along the taskbar into the notch:
    /// "click" = when you click the screen, "manual" = only when you send it back.
    pub leave: String,
    /// The day it was "born" (YYYY-MM-DD), set on first launch.
    pub born: String,
    pub pats: u32,
    /// Grows with focus sessions, breaks and finished tasks.
    pub xp: u32,
    /// Treats given while it sits on the desktop.
    pub treats: u32,
    /// Called out of the notch: it walks the taskbar until it goes back.
    pub buddy_out: bool,
    /// No longer used (the pet isn't placed on the desktop any more); kept so older settings load.
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub buddy_x: Option<i32>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub buddy_y: Option<i32>,
}

impl Default for PetSettings {
    fn default() -> Self {
        Self {
            name: String::new(),
            color: String::new(),
            tone: "cheerful".into(),
            talk: true,
            chattiness: "normal".into(),
            chat_minutes: 5,
            say_meetings: true,
            say_breaks: true,
            say_focus: true,
            say_agents: true,
            say_battery: true,
            say_weather: true,
            say_late: true,
            say_greeting: true,
            react_music: true,
            react_video: true,
            react_coding: true,
            react_weather: true,
            react_battery: true,
            react_day: true,
            petting: true,
            leave: "click".into(),
            born: String::new(),
            pats: 0,
            xp: 0,
            treats: 0,
            buddy_out: false,
            buddy_x: None,
            buddy_y: None,
        }
    }
}

pub const PET_TONES: [&str; 7] = [
    "cheerful",
    "sarcastic",
    "professional",
    "calm",
    "coach",
    "nerdy",
    "dramatic",
];

/// Healthy breaks and the streak the pet keeps.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export)]
pub struct BreakSettings {
    pub no_eyes: bool,
    pub no_stretch: bool,
    pub no_water: bool,
    /// Minutes at the PC between reminders.
    pub eye_min: u32,
    pub stretch_min: u32,
    pub water_min: u32,
    /// Breaks taken on `day` (YYYY-MM-DD).
    pub day: String,
    pub count: u32,
    /// Days in a row with three breaks, the last of them `counted_day`.
    pub streak: u32,
    pub counted_day: String,
    pub best: u32,
}

impl Default for BreakSettings {
    fn default() -> Self {
        Self {
            no_eyes: false,
            no_stretch: false,
            no_water: true,
            eye_min: 20,
            stretch_min: 60,
            water_min: 90,
            day: String::new(),
            count: 0,
            streak: 0,
            counted_day: String::new(),
            best: 0,
        }
    }
}

/// Fun extras: the pet walking the taskbar and PC lights following the notch.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export)]
pub struct FunSettings {
    /// The pet walks along the taskbar while you're away.
    pub pet_walk: bool,
    /// Light up PC RGB through OpenRGB.
    pub rgb: bool,
    /// OpenRGB's SDK server port (0 = 6742).
    pub rgb_port: u16,
}

/// Desk routines.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export)]
pub struct DeskSettings {
    pub routines: Vec<crate::desk::Routine>,
}

/// Developer tools.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export)]
pub struct DevSettings {
    /// Custom widgets (URL or command).
    pub widgets: Vec<crate::developer::WidgetConfig>,
    /// No note when a dev server stops.
    pub no_server_alerts: bool,
    /// Don't count coding time.
    pub no_coding_tracker: bool,
}

/// The password vault in the user's Obsidian vault.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export)]
pub struct VaultSettings {
    /// The Obsidian vault folder (logins go in its "Notch Vault" subfolder).
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub folder: Option<String>,
    /// Ask for Windows Hello before showing or copying a password.
    pub require_hello: bool,
    /// Seconds before a copied password or code is cleared (0 = 30).
    pub clear_secs: u32,
    /// Never clear copied passwords and codes (they still stay out of clipboard history).
    pub never_clear: bool,
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

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize, TS)]
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
    pub github_alerts: GithubAlertSettings,
    pub music: MusicSettings,
    pub focus: FocusSettings,
    pub game: GameSettings,
    pub appearance: AppearanceSettings,
    pub behavior: BehaviorSettings,
    pub shortcuts: ShortcutSettings,
    pub gestures: GestureSettings,
    pub dock: DockSettings,
    pub home: HomeSettings,
    pub vault: VaultSettings,
    pub dev: DevSettings,
    pub day: DaySettings,
    pub desk: DeskSettings,
    pub breaks: BreakSettings,
    pub pet: PetSettings,
    pub ai: AiSettings,
    pub fun: FunSettings,
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
        let f = &self.focus;
        if [f.rain, f.wind, f.ocean, f.fire, f.brown, f.pink, f.master]
            .iter()
            .any(|v| !v.is_finite() || !(0.0..=1.0).contains(v))
        {
            return Err(AppError::InvalidInput("invalid sound level".into()));
        }
        if self.game.games.len() + self.game.not_games.len() > 200
            || self
                .game
                .games
                .iter()
                .chain(&self.game.not_games)
                .any(|g| g.is_empty() || g.len() > 100)
        {
            return Err(AppError::InvalidInput("invalid games list".into()));
        }
        if self.vault.clear_secs > 600 || self.vault.folder.as_ref().is_some_and(|f| f.len() > 1000) {
            return Err(AppError::InvalidInput("invalid vault settings".into()));
        }
        if self.desk.routines.len() > crate::desk::MAX_ROUTINES
            || self
                .desk
                .routines
                .iter()
                .any(|r| !valid_id(&r.id) || crate::desk::validate(r).is_err())
        {
            return Err(AppError::InvalidInput("invalid routine".into()));
        }
        let ai = &self.ai;
        if !["claude", "codex", "gemini", "ollama", "openai"].contains(&ai.provider.as_str())
            || ai.models.len() > 10
            || ai.models.values().any(|m| m.len() > 200)
            || ai.base_url.len() > 500
            || !(ai.base_url.is_empty()
                || ai.base_url.starts_with("https://")
                || ai.base_url.starts_with("http://"))
        {
            return Err(AppError::InvalidInput("invalid AI settings".into()));
        }
        let pet = &self.pet;
        if pet.name.chars().count() > 24
            || !(pet.color.is_empty() || valid_hex_color(&pet.color))
            || !PET_TONES.contains(&pet.tone.as_str())
            || !["quiet", "normal", "chatty"].contains(&pet.chattiness.as_str())
            || pet.born.len() > 10
        {
            return Err(AppError::InvalidInput("invalid pet settings".into()));
        }
        let br = &self.breaks;
        if [br.eye_min, br.stretch_min, br.water_min]
            .iter()
            .any(|m| !(5..=480).contains(m))
            || br.day.len() > 10
            || br.counted_day.len() > 10
        {
            return Err(AppError::InvalidInput("invalid break settings".into()));
        }
        let d = &self.day;
        if d.start_min >= 24 * 60
            || d.end_min > 24 * 60
            || d.start_min >= d.end_min
            || d.work_days.len() > 7
            || d.work_days.iter().any(|w| !(1..=7).contains(w))
        {
            return Err(AppError::InvalidInput("invalid workday".into()));
        }
        let w = &self.dev.widgets;
        if w.len() > 20
            || w.iter().any(|w| {
                w.name.trim().is_empty()
                    || w.name.len() > 60
                    || w.source.trim().is_empty()
                    || w.source.len() > 2000
                    || !valid_id(&w.id)
                    || !["url", "command"].contains(&w.kind.as_str())
                    || (w.kind == "url"
                        && !(w.source.starts_with("https://") || w.source.starts_with("http://")))
                    || !(10..=86_400).contains(&w.refresh_secs)
            })
        {
            return Err(AppError::InvalidInput("invalid widget".into()));
        }
        if self.hud.charge_alert_at > 100 {
            return Err(AppError::InvalidInput("invalid charge level".into()));
        }
        let a = &self.appearance;
        if !(40..=100).contains(&a.opacity)
            || a.roundness > 100
            || !(-800..=800).contains(&a.offset_x)
            || a.offset_y > 80
            || !(a.accent.is_empty() || valid_hex_color(&a.accent))
        {
            return Err(AppError::InvalidInput("invalid look settings".into()));
        }
        let b = &self.behavior;
        if b.auto_close_secs > 600
            || !b.sound_volume.is_finite()
            || !(0.0..=1.0).contains(&b.sound_volume)
            || b.quiet.start_min >= 24 * 60
            || b.quiet.end_min >= 24 * 60
        {
            return Err(AppError::InvalidInput("invalid behavior settings".into()));
        }
        let k = &self.shortcuts;
        if [&k.open, &k.ask, &k.clipboard, &k.grab, &k.pin, &k.capture]
            .iter()
            .any(|s| s.len() > 40)
        {
            return Err(AppError::InvalidInput("invalid shortcut".into()));
        }
        let mut ids = self
            .dock
            .order
            .iter()
            .chain(&self.dock.hidden)
            .chain(self.dock.first.iter())
            .chain(&self.home.hidden_tiles);
        if self.dock.order.len() + self.dock.hidden.len() + self.home.hidden_tiles.len() > 120
            || ids.any(|id| !valid_id(id))
        {
            return Err(AppError::InvalidInput("invalid dock or home settings".into()));
        }
        if self.calendar.reminder_minutes > 120 {
            return Err(AppError::InvalidInput("reminder lead is too long".into()));
        }
        for id in self.modules.keys() {
            if !valid_id(id) {
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

fn valid_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 64
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

fn valid_hex_color(s: &str) -> bool {
    s.len() == 7 && s.starts_with('#') && s[1..].chars().all(|c| c.is_ascii_hexdigit())
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
            // One unreadable value must not wipe everything: keep every group that still reads,
            // reset only the rest, and keep the original row so nothing is lost for good.
            tracing::warn!(%error, "some stored settings are unreadable; keeping the rest");
            let _ = conn.execute(
                "INSERT INTO settings (key, value, updated_at) VALUES (?1, ?2, ?3)
                 ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at",
                params![UNREADABLE_KEY, raw, chrono::Utc::now().to_rfc3339()],
            );
            Ok(salvage(&raw))
        }
    }
}

/// Where an unreadable settings row is kept (for recovery) before it's repaired.
const UNREADABLE_KEY: &str = "app.unreadable";

/// Settings from JSON that doesn't fully parse: every top-level group that still reads is
/// kept, the others take their defaults. Returns the names of the groups that were reset.
pub fn salvage_groups(raw: &str) -> (Settings, Vec<String>) {
    let Ok(serde_json::Value::Object(stored)) = serde_json::from_str::<serde_json::Value>(raw) else {
        return (Settings::default(), vec!["all".into()]);
    };
    let Ok(serde_json::Value::Object(mut merged)) = serde_json::to_value(Settings::default()) else {
        return (Settings::default(), vec!["all".into()]);
    };
    let mut reset = Vec::new();
    for (key, value) in stored {
        let mut candidate = merged.clone();
        candidate.insert(key.clone(), value.clone());
        if serde_json::from_value::<Settings>(serde_json::Value::Object(candidate)).is_ok() {
            merged.insert(key, value);
        } else {
            reset.push(key);
        }
    }
    let settings = serde_json::from_value(serde_json::Value::Object(merged)).unwrap_or_default();
    (settings, reset)
}

fn salvage(raw: &str) -> Settings {
    let (settings, reset) = salvage_groups(raw);
    tracing::warn!(groups = ?reset, "settings groups reset to defaults");
    settings
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

    #[test]
    fn one_bad_value_doesnt_reset_everything() {
        let mut good = Settings::default();
        good.appearance.bubble = BubbleStyle::Dot;
        good.music.hide_border = true;
        let mut json: serde_json::Value = serde_json::to_value(&good).unwrap();
        // A value an older or newer Notch wrote that this one can't read.
        json["behavior"]["fullscreen"] = serde_json::json!("somethingNew");
        let (settings, reset) = salvage_groups(&json.to_string());
        assert_eq!(reset, ["behavior"]);
        assert_eq!(
            settings.appearance.bubble,
            BubbleStyle::Dot,
            "other groups are kept"
        );
        assert!(settings.music.hide_border);
        assert_eq!(settings.behavior, BehaviorSettings::default());
        assert_eq!(salvage_groups("not json").1, ["all"]);
    }

    #[test]
    fn unreadable_settings_are_salvaged_and_backed_up() {
        let mut conn = rusqlite::Connection::open_in_memory().unwrap();
        crate::db::migrate_for_tests(&mut conn);
        let mut stored: serde_json::Value = serde_json::to_value(Settings::default()).unwrap();
        stored["music"]["hideBorder"] = serde_json::json!(true);
        stored["behavior"]["fullscreen"] = serde_json::json!(42);
        conn.execute(
            "INSERT INTO settings (key, value, updated_at) VALUES (?1, ?2, 'x')",
            params![SETTINGS_KEY, stored.to_string()],
        )
        .unwrap();
        let loaded = load(&conn).unwrap();
        assert!(loaded.music.hide_border);
        let backup: String = conn
            .query_row(
                "SELECT value FROM settings WHERE key = ?1",
                [UNREADABLE_KEY],
                |r| r.get(0),
            )
            .unwrap();
        assert!(backup.contains("42"));
    }
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
    fn new_groups_have_sensible_defaults() {
        let parsed: Settings = serde_json::from_str("{}").unwrap();
        assert_eq!(parsed.appearance.opacity, 100);
        assert_eq!(parsed.appearance.roundness, 100);
        assert_eq!(parsed.shortcuts.open, "Ctrl+Alt+N");
        assert_eq!(parsed.behavior.quiet.start_min, 22 * 60);
        assert!(parsed.validate().is_ok());
        assert!(Settings::default().validate().is_ok());
    }

    #[test]
    fn rejects_out_of_range_look_and_behavior() {
        let mut s = Settings::default();
        s.appearance.opacity = 10;
        assert!(s.validate().is_err());
        let mut s = Settings::default();
        s.appearance.accent = "red; background:url(x)".into();
        assert!(s.validate().is_err());
        s.appearance.accent = "#1a2B3c".into();
        assert!(s.validate().is_ok());
        let mut s = Settings::default();
        s.behavior.quiet.end_min = 24 * 60;
        assert!(s.validate().is_err());
        let mut s = Settings::default();
        s.dock.order.push("../x".into());
        assert!(s.validate().is_err());
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
