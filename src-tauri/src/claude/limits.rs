//! Claude plan usage limits (5-hour and weekly), from Claude Code's status line.
//!
//! Claude Code passes `rate_limits` to a configured status-line command for claude.ai Pro and
//! Max subscribers. Notch's status line forwards that JSON to the local hook server; only the
//! limit numbers are kept — model, folder, cost, and transcript path are ignored.

use chrono::{DateTime, TimeZone, Utc};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct LimitWindow {
    /// 0–100 (Claude Code's own figure).
    pub used_percent: f64,
    pub resets_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct PlanLimits {
    #[ts(optional)]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub five_hour: Option<LimitWindow>,
    #[ts(optional)]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seven_day: Option<LimitWindow>,
    /// When Claude Code last reported these numbers.
    pub updated_at: DateTime<Utc>,
}

/// Sample limits for demo mode (recording the website's videos).
pub fn demo(now: DateTime<Utc>) -> PlanLimits {
    PlanLimits {
        five_hour: Some(LimitWindow {
            used_percent: 42.0,
            resets_at: now + chrono::Duration::minutes(134),
        }),
        seven_day: Some(LimitWindow {
            used_percent: 68.0,
            resets_at: now + chrono::Duration::days(3),
        }),
        updated_at: now,
    }
}

#[derive(Deserialize)]
struct RawWindow {
    used_percentage: Option<f64>,
    resets_at: Option<i64>,
}

#[derive(Deserialize)]
struct RawLimits {
    five_hour: Option<RawWindow>,
    seven_day: Option<RawWindow>,
}

/// The only part of the status-line input Notch reads.
#[derive(Deserialize)]
struct StatusLineInput {
    rate_limits: Option<RawLimits>,
}

fn window(raw: Option<RawWindow>) -> Option<LimitWindow> {
    let raw = raw?;
    let used = raw.used_percentage.filter(|p| p.is_finite())?;
    let resets_at = Utc.timestamp_opt(raw.resets_at?, 0).single()?;
    Some(LimitWindow {
        used_percent: used.clamp(0.0, 100.0),
        resets_at,
    })
}

/// Limits from one status-line update, or `None` if it carried none (API-key users, or the
/// first update of a session before any response).
pub fn parse(body: &[u8], now: DateTime<Utc>) -> Option<PlanLimits> {
    let input: StatusLineInput = serde_json::from_slice(body).ok()?;
    let raw = input.rate_limits?;
    let limits = PlanLimits {
        five_hour: window(raw.five_hour),
        seven_day: window(raw.seven_day),
        updated_at: now,
    };
    (limits.five_hour.is_some() || limits.seven_day.is_some()).then_some(limits)
}

impl PlanLimits {
    /// Drop windows whose reset time has passed: their usage restarted and the old figure no
    /// longer means anything. `None` when nothing current is left.
    pub fn current(self, now: DateTime<Utc>) -> Option<Self> {
        let live = |w: Option<LimitWindow>| w.filter(|w| w.resets_at > now);
        let limits = Self {
            five_hour: live(self.five_hour),
            seven_day: live(self.seven_day),
            ..self
        };
        (limits.five_hour.is_some() || limits.seven_day.is_some()).then_some(limits)
    }
}

/// The text Claude Code shows as its status line, e.g. `5h 23% · week 41%`.
pub fn status_text(limits: Option<&PlanLimits>) -> String {
    let Some(limits) = limits else {
        return String::new();
    };
    let mut parts = Vec::new();
    if let Some(w) = limits.five_hour {
        parts.push(format!("5h {:.0}%", w.used_percent));
    }
    if let Some(w) = limits.seven_day {
        parts.push(format!("week {:.0}%", w.used_percent));
    }
    parts.join(" · ")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn now() -> DateTime<Utc> {
        Utc.timestamp_opt(1_738_400_000, 0).unwrap()
    }

    #[test]
    fn keeps_only_the_limit_numbers() {
        let body = br#"{
            "session_id": "s1", "cwd": "C:\\secret\\project", "cost": { "total_cost_usd": 1.5 },
            "rate_limits": {
                "five_hour": { "used_percentage": 23.5, "resets_at": 1738425600 },
                "seven_day": { "used_percentage": 41.2, "resets_at": 1738857600 }
            }
        }"#;
        let limits = parse(body, now()).unwrap();
        assert_eq!(limits.five_hour.unwrap().used_percent, 23.5);
        assert_eq!(limits.seven_day.unwrap().resets_at.timestamp(), 1_738_857_600);
        assert_eq!(status_text(Some(&limits)), "5h 24% · week 41%");
        let json = serde_json::to_string(&limits).unwrap();
        assert!(!json.contains("secret") && !json.contains("cost"));
    }

    #[test]
    fn handles_missing_and_partial_limits() {
        assert!(parse(br#"{"model":{"id":"x"}}"#, now()).is_none());
        assert!(parse(br#"{"rate_limits":{}}"#, now()).is_none());
        assert!(parse(b"not json", now()).is_none());
        let only_week = parse(
            br#"{"rate_limits":{"seven_day":{"used_percentage":150,"resets_at":1738857600}}}"#,
            now(),
        )
        .unwrap();
        assert!(only_week.five_hour.is_none());
        assert_eq!(only_week.seven_day.unwrap().used_percent, 100.0);
        assert_eq!(status_text(Some(&only_week)), "week 100%");
        assert_eq!(status_text(None), "");
    }

    #[test]
    fn drops_windows_that_have_reset() {
        let limits = parse(
            br#"{"rate_limits":{
                "five_hour":{"used_percentage":90,"resets_at":1738403600},
                "seven_day":{"used_percentage":40,"resets_at":1738857600}}}"#,
            now(),
        )
        .unwrap();
        let later = now() + chrono::Duration::hours(2);
        let current = limits.current(later).unwrap();
        assert!(current.five_hour.is_none());
        assert!(current.seven_day.is_some());
        assert!(limits.current(now() + chrono::Duration::days(30)).is_none());
    }
}
