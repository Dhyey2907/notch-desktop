//! Claude Code activity and usage (architecture.md §7, plan.md Phase 9).
//!
//! Two independent sources, neither of which needs network access:
//! - `transcripts` + `usage`: local estimates of usage from Claude Code's own transcript files;
//! - `hooks`: live "what is Claude doing right now" from Claude Code's HTTP hooks.
//!
//! Official account quotas aren't available to local apps, so none are shown or invented.

pub mod agents;
pub mod approvals;
pub mod connect;
pub mod hooks;
pub mod limits;
pub mod process;
pub mod relay;
pub mod service;
pub mod transcripts;
pub mod usage;

use chrono::{DateTime, Utc};
use serde::Serialize;
use ts_rs::TS;

use hooks::AgentSession;
use limits::PlanLimits;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct AgentLink {
    pub agent: agents::Agent,
    pub name: String,
    /// Installed on this PC (its program or its config folder was found).
    pub detected: bool,
    /// Notch's hooks are in its config.
    pub connected: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct HooksStatus {
    /// Notch's hooks are present in Claude Code's settings.
    pub installed: bool,
    /// The local hook server is accepting events.
    pub listening: bool,
    /// Installed by an older Notch; reconnect to add newer events (e.g. approvals).
    pub outdated: bool,
    /// Notch's status line is set in Claude Code, forwarding plan limits.
    pub status_line: bool,
    /// Another status line is already configured, so Notch won't add its own.
    pub status_line_taken: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct IndexStatus {
    /// `~/.claude/projects` exists and has transcripts.
    pub transcripts_found: bool,
    /// The first full scan has finished.
    pub ready: bool,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_indexed_at: Option<DateTime<Utc>>,
}

/// The most recent rate-limit event recorded by Claude Code, while it's still in effect.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct QuotaStatus {
    /// e.g. "five_hour", "seven_day".
    pub limit_type: String,
    pub resets_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ClaudeStatus {
    /// Other coding agents Notch can connect to.
    pub agents: Vec<AgentLink>,
    pub live: Vec<AgentSession>,
    pub hooks: HooksStatus,
    pub index: IndexStatus,
    /// Last plan limits reported by Claude Code's status line, while still current.
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limits: Option<PlanLimits>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quota: Option<QuotaStatus>,
}
