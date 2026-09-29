//! Google Calendar API adapter: fetches the primary calendar's events and maps them to
//! provider-neutral `CalendarEvent`s. Provider JSON never leaves this file (architecture.md §8).

use chrono::{DateTime, NaiveDate, Utc};
use serde::Deserialize;

use super::{local_midnight, CalendarEvent};
use crate::error::{AppError, AppResult};

const EVENTS_URL: &str = "https://www.googleapis.com/calendar/v3/calendars/primary/events";
/// Safety cap on events per sync; a week of a busy calendar is far below this.
const MAX_RESULTS: &str = "250";

#[derive(Debug, Deserialize)]
struct EventList {
    #[serde(default)]
    items: Vec<RawEvent>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawEvent {
    id: String,
    #[serde(default)]
    status: Option<String>,
    #[serde(default)]
    summary: Option<String>,
    start: RawTime,
    end: RawTime,
    #[serde(default)]
    hangout_link: Option<String>,
    #[serde(default)]
    conference_data: Option<ConferenceData>,
    #[serde(default)]
    location: Option<String>,
    #[serde(default)]
    html_link: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawTime {
    #[serde(default)]
    date_time: Option<DateTime<Utc>>,
    /// All-day events carry a date only (end date is exclusive).
    #[serde(default)]
    date: Option<NaiveDate>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ConferenceData {
    #[serde(default)]
    entry_points: Vec<EntryPoint>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct EntryPoint {
    #[serde(default)]
    entry_point_type: Option<String>,
    #[serde(default)]
    uri: Option<String>,
}

fn https(url: Option<&str>) -> Option<String> {
    url.filter(|u| u.starts_with("https://")).map(str::to_owned)
}

impl RawTime {
    fn resolve(&self) -> Option<(DateTime<Utc>, bool)> {
        match (self.date_time, self.date) {
            (Some(t), _) => Some((t, false)),
            (None, Some(d)) => Some((local_midnight(d), true)),
            (None, None) => None,
        }
    }
}

/// Map Google's event JSON to our events, dropping cancelled or malformed entries.
/// Treated as untrusted input: only https links are kept (architecture.md §11).
fn map_events(list: EventList) -> Vec<CalendarEvent> {
    let mut events: Vec<CalendarEvent> = list
        .items
        .into_iter()
        .filter(|e| e.status.as_deref() != Some("cancelled"))
        .filter_map(|e| {
            let (start, all_day) = e.start.resolve()?;
            let (end, _) = e.end.resolve()?;
            let video = e.conference_data.as_ref().and_then(|c| {
                c.entry_points
                    .iter()
                    .find(|p| p.entry_point_type.as_deref() == Some("video"))
                    .and_then(|p| https(p.uri.as_deref()))
            });
            let join_url = https(e.hangout_link.as_deref())
                .or(video)
                .or_else(|| https(e.location.as_deref()));
            Some(CalendarEvent {
                title: e
                    .summary
                    .filter(|s| !s.trim().is_empty())
                    .unwrap_or_else(|| "(No title)".into()),
                id: e.id,
                start,
                end: end.max(start),
                all_day,
                join_url,
                location: e.location.filter(|l| !l.starts_with("https://")),
                html_link: https(e.html_link.as_deref()),
            })
        })
        .collect();
    events.sort_by_key(|e| (e.start, !e.all_day));
    events
}

pub fn parse_events(json: &str) -> AppResult<Vec<CalendarEvent>> {
    let list: EventList = serde_json::from_str(json).map_err(|e| AppError::InvalidInput(e.to_string()))?;
    Ok(map_events(list))
}

/// Errors from the API that decide how the service reacts.
#[derive(Debug)]
pub enum FetchError {
    /// 401: the access token is no longer valid.
    Unauthorized,
    Network(String),
    Other(String),
}

pub async fn fetch_events(
    http: &reqwest::Client,
    access_token: &str,
    time_min: DateTime<Utc>,
    time_max: DateTime<Utc>,
) -> Result<Vec<CalendarEvent>, FetchError> {
    let response = http
        .get(EVENTS_URL)
        .bearer_auth(access_token)
        .query(&[
            ("timeMin", time_min.to_rfc3339().as_str()),
            ("timeMax", time_max.to_rfc3339().as_str()),
            ("singleEvents", "true"),
            ("orderBy", "startTime"),
            ("maxResults", MAX_RESULTS),
        ])
        .send()
        .await
        .map_err(|e| FetchError::Network(e.without_url().to_string()))?;
    match response.status() {
        s if s == reqwest::StatusCode::UNAUTHORIZED => Err(FetchError::Unauthorized),
        s if !s.is_success() => Err(FetchError::Other(format!("calendar API returned {s}"))),
        _ => {
            let body = response
                .text()
                .await
                .map_err(|e| FetchError::Network(e.without_url().to_string()))?;
            parse_events(&body).map_err(|e| FetchError::Other(e.to_string()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_timed_all_day_and_cancelled_events() {
        let json = r#"{
          "items": [
            { "id": "b", "summary": "Design Review", "status": "confirmed",
              "start": { "dateTime": "2026-09-28T15:00:00+05:30" },
              "end": { "dateTime": "2026-09-28T15:30:00+05:30" },
              "hangoutLink": "https://meet.google.com/abc-defg-hij",
              "htmlLink": "https://www.google.com/calendar/event?eid=x" },
            { "id": "a", "summary": "Holiday",
              "start": { "date": "2026-09-28" }, "end": { "date": "2026-09-29" } },
            { "id": "c", "summary": "Gone", "status": "cancelled",
              "start": { "dateTime": "2026-09-28T09:00:00Z" }, "end": { "dateTime": "2026-09-28T10:00:00Z" } }
          ]
        }"#;
        let events = parse_events(json).unwrap();
        assert_eq!(events.len(), 2);
        let review = events.iter().find(|e| e.id == "b").unwrap();
        assert!(!review.all_day);
        assert_eq!(review.start.to_rfc3339(), "2026-09-28T09:30:00+00:00");
        assert_eq!(
            review.join_url.as_deref(),
            Some("https://meet.google.com/abc-defg-hij")
        );
        assert!(events.iter().find(|e| e.id == "a").unwrap().all_day);
    }

    #[test]
    fn finds_join_links_in_conference_data_or_location_but_only_https() {
        let json = r#"{ "items": [
            { "id": "zoom", "summary": "Zoom call",
              "start": { "dateTime": "2026-09-28T10:00:00Z" }, "end": { "dateTime": "2026-09-28T11:00:00Z" },
              "conferenceData": { "entryPoints": [
                { "entryPointType": "phone", "uri": "tel:+1-555" },
                { "entryPointType": "video", "uri": "https://zoom.us/j/123" } ] } },
            { "id": "teams", "summary": "Teams",
              "start": { "dateTime": "2026-09-28T12:00:00Z" }, "end": { "dateTime": "2026-09-28T13:00:00Z" },
              "location": "https://teams.microsoft.com/l/meetup-join/x" },
            { "id": "sketchy", "summary": "Sketchy",
              "start": { "dateTime": "2026-09-28T14:00:00Z" }, "end": { "dateTime": "2026-09-28T15:00:00Z" },
              "hangoutLink": "javascript:alert(1)", "location": "Room 4" }
        ] }"#;
        let events = parse_events(json).unwrap();
        let by_id = |id: &str| events.iter().find(|e| e.id == id).unwrap();
        assert_eq!(by_id("zoom").join_url.as_deref(), Some("https://zoom.us/j/123"));
        assert_eq!(
            by_id("teams").join_url.as_deref(),
            Some("https://teams.microsoft.com/l/meetup-join/x")
        );
        assert_eq!(by_id("teams").location, None);
        assert_eq!(by_id("sketchy").join_url, None);
        assert_eq!(by_id("sketchy").location.as_deref(), Some("Room 4"));
    }

    #[test]
    fn tolerates_missing_fields_and_bad_times() {
        let json = r#"{ "items": [
            { "id": "untitled", "start": { "dateTime": "2026-09-28T10:00:00Z" }, "end": { "dateTime": "2026-09-28T09:00:00Z" } },
            { "id": "broken", "start": {}, "end": {} }
        ] }"#;
        let events = parse_events(json).unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].title, "(No title)");
        assert_eq!(events[0].end, events[0].start); // end before start is clamped
        assert!(parse_events("not json").is_err());
        assert!(parse_events("{}").unwrap().is_empty());
    }
}
