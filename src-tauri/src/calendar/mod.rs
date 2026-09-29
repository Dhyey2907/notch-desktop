//! Calendar module backend (architecture.md §7 Calendar, plan.md Phase 7).
//!
//! `google` talks to the Google Calendar API, `oauth` handles the installed-app sign-in,
//! and `service` owns sync, the local cache, and reminder timing. Everything the UI sees
//! arrives as a `CalendarSnapshot` or a `ReminderDue` event.

pub mod google;
pub mod oauth;
pub mod service;

use std::collections::HashSet;

use chrono::{DateTime, Duration, Local, NaiveDate, TimeZone, Utc};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct CalendarEvent {
    pub id: String,
    pub title: String,
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
    pub all_day: bool,
    /// Meeting link (Meet, or an https conference / location link), if any.
    #[ts(optional)]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub join_url: Option<String>,
    #[ts(optional)]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub location: Option<String>,
    /// Link to the event in Google Calendar.
    #[ts(optional)]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub html_link: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum CalendarErrorKind {
    /// Access was revoked or expired; the user must reconnect.
    AuthExpired,
    /// Couldn't reach Google; cached events are still shown.
    Network,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(tag = "state", rename_all = "camelCase")]
#[ts(export)]
pub enum CalendarStatus {
    /// No OAuth client set up yet.
    NotConfigured,
    Disconnected,
    Connecting,
    Connected,
    Error {
        reason: CalendarErrorKind,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct CalendarSnapshot {
    pub status: CalendarStatus,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub account: Option<String>,
    /// Events from the start of today through the next week, sorted by start.
    pub events: Vec<CalendarEvent>,
    /// When events were last fetched; the UI flags stale data.
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_synced_at: Option<DateTime<Utc>>,
    pub mock: bool,
}

/// Days of events kept in view and in the cache.
pub const WINDOW_DAYS: i64 = 8;

/// `[local midnight today, +WINDOW_DAYS)` in UTC.
pub fn sync_window(now: DateTime<Utc>) -> (DateTime<Utc>, DateTime<Utc>) {
    let today = now.with_timezone(&Local).date_naive();
    let start = local_midnight(today);
    (start, start + Duration::days(WINDOW_DAYS))
}

/// Local midnight of `date` as UTC (falls back to UTC midnight across odd DST gaps).
pub fn local_midnight(date: NaiveDate) -> DateTime<Utc> {
    let naive = date.and_hms_opt(0, 0, 0).expect("midnight is valid");
    Local
        .from_local_datetime(&naive)
        .earliest()
        .map(|t| t.with_timezone(&Utc))
        .unwrap_or_else(|| Utc.from_utc_datetime(&naive))
}

/// Identity of one reminder, so it fires once even if the event is re-synced.
pub fn reminder_key(event: &CalendarEvent) -> String {
    format!("{}@{}", event.id, event.start.timestamp())
}

/// Reminders due now, and when the next one is due (for sleeping until then).
/// All-day events never get countdown reminders.
pub fn reminders<'a>(
    events: &'a [CalendarEvent],
    lead: Duration,
    now: DateTime<Utc>,
    fired: &HashSet<String>,
) -> (Vec<&'a CalendarEvent>, Option<DateTime<Utc>>) {
    let mut due = Vec::new();
    let mut next: Option<DateTime<Utc>> = None;
    for event in events.iter().filter(|e| !e.all_day && e.start > now) {
        if fired.contains(&reminder_key(event)) {
            continue;
        }
        let at = event.start - lead;
        if at <= now {
            due.push(event);
        } else {
            next = Some(next.map_or(at, |n| n.min(at)));
        }
    }
    (due, next)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn event(id: &str, start: DateTime<Utc>, all_day: bool) -> CalendarEvent {
        CalendarEvent {
            id: id.into(),
            title: id.into(),
            start,
            end: start + Duration::minutes(30),
            all_day,
            join_url: None,
            location: None,
            html_link: None,
        }
    }

    #[test]
    fn reminders_fire_inside_the_lead_window_once() {
        let now = Utc.with_ymd_and_hms(2026, 9, 28, 10, 0, 0).unwrap();
        let events = vec![
            event("soon", now + Duration::minutes(3), false),
            event("later", now + Duration::minutes(40), false),
            event("allday", now + Duration::minutes(2), true),
            event("past", now - Duration::minutes(1), false),
        ];
        let lead = Duration::minutes(5);

        let (due, next) = reminders(&events, lead, now, &HashSet::new());
        assert_eq!(due.iter().map(|e| e.id.as_str()).collect::<Vec<_>>(), ["soon"]);
        assert_eq!(next, Some(now + Duration::minutes(35)));

        let fired: HashSet<_> = [reminder_key(&events[0])].into();
        let (due, _) = reminders(&events, lead, now, &fired);
        assert!(due.is_empty());
    }

    #[test]
    fn a_rescheduled_event_gets_a_new_reminder() {
        let now = Utc.with_ymd_and_hms(2026, 9, 28, 10, 0, 0).unwrap();
        let original = event("e", now + Duration::minutes(3), false);
        let fired: HashSet<_> = [reminder_key(&original)].into();
        let moved = event("e", now + Duration::minutes(4), false);
        let (due, _) = reminders(std::slice::from_ref(&moved), Duration::minutes(5), now, &fired);
        assert_eq!(due.len(), 1);
    }

    #[test]
    fn window_starts_at_local_midnight_and_spans_the_week() {
        let now = Utc::now();
        let (start, end) = sync_window(now);
        assert!(start <= now && now < start + Duration::days(1));
        assert_eq!(end - start, Duration::days(WINDOW_DAYS));
    }

    #[test]
    fn status_serializes_as_a_tagged_union() {
        let json = serde_json::to_value(CalendarStatus::Error {
            reason: CalendarErrorKind::AuthExpired,
        })
        .unwrap();
        assert_eq!(
            json,
            serde_json::json!({ "state": "error", "reason": "authExpired" })
        );
    }
}
