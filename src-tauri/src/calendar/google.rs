//! Google Calendar API adapter: fetches the primary calendar's events and maps them to
//! provider-neutral `CalendarEvent`s. Provider JSON never leaves this file (architecture.md §8).

use chrono::{DateTime, NaiveDate, Utc};
use serde::Deserialize;

use super::{local_midnight, CalendarEvent, InviteResponse, NewEvent};
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
    #[serde(default)]
    attendees: Vec<RawAttendee>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawAttendee {
    #[serde(default, rename = "self")]
    is_self: bool,
    #[serde(default)]
    organizer: bool,
    #[serde(default)]
    response_status: Option<String>,
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
            // Only invites from someone else carry an answer of yours.
            let my_response = e
                .attendees
                .iter()
                .find(|a| a.is_self && !a.organizer)
                .and_then(|a| a.response_status.as_deref())
                .and_then(InviteResponse::from_api);
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
                my_response,
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

fn status_error(status: reqwest::StatusCode) -> FetchError {
    if status == reqwest::StatusCode::UNAUTHORIZED {
        FetchError::Unauthorized
    } else {
        FetchError::Other(format!("calendar API returned {status}"))
    }
}

/// Google's event JSON for a new event: timed events in UTC, all-day ones by local date
/// (Google's all-day end date is exclusive).
pub fn new_event_body(event: &NewEvent) -> serde_json::Value {
    let (start, end) = if event.all_day {
        let first = event.start.with_timezone(&chrono::Local).date_naive();
        let last = (event.end - chrono::Duration::seconds(1))
            .with_timezone(&chrono::Local)
            .date_naive()
            .max(first);
        (
            serde_json::json!({ "date": first.to_string() }),
            serde_json::json!({ "date": last.succ_opt().unwrap_or(last).to_string() }),
        )
    } else {
        (
            serde_json::json!({ "dateTime": event.start.to_rfc3339() }),
            serde_json::json!({ "dateTime": event.end.to_rfc3339() }),
        )
    };
    let mut body = serde_json::json!({ "summary": event.title.trim(), "start": start, "end": end });
    if let Some(location) = event.location.as_deref().map(str::trim).filter(|l| !l.is_empty()) {
        body["location"] = location.into();
    }
    body
}

pub async fn create_event(http: &reqwest::Client, token: &str, event: &NewEvent) -> Result<(), FetchError> {
    let response = http
        .post(EVENTS_URL)
        .bearer_auth(token)
        .json(&new_event_body(event))
        .send()
        .await
        .map_err(|e| FetchError::Network(e.without_url().to_string()))?;
    if response.status().is_success() {
        Ok(())
    } else {
        Err(status_error(response.status()))
    }
}

/// Google turns a sentence ("Lunch with Ann tomorrow 1pm") into an event. Returns its title
/// and start so the notch can say what it made.
pub async fn quick_add(
    http: &reqwest::Client,
    token: &str,
    text: &str,
) -> Result<(String, Option<DateTime<Utc>>), FetchError> {
    let response = http
        .post(format!("{EVENTS_URL}/quickAdd"))
        .bearer_auth(token)
        .query(&[("text", text)])
        .send()
        .await
        .map_err(|e| FetchError::Network(e.without_url().to_string()))?;
    if !response.status().is_success() {
        return Err(status_error(response.status()));
    }
    let made: RawEvent = response
        .json()
        .await
        .map_err(|e| FetchError::Other(e.without_url().to_string()))?;
    Ok((
        made.summary.unwrap_or_else(|| "(No title)".into()),
        made.start.resolve().map(|(t, _)| t),
    ))
}

/// Answer an invite: the event's attendee list with your own answer changed. The organizer is
/// told, as when you answer in Google Calendar.
pub async fn respond(
    http: &reqwest::Client,
    token: &str,
    event_id: &str,
    answer: InviteResponse,
) -> Result<(), FetchError> {
    let url = format!("{EVENTS_URL}/{event_id}");
    let response = http
        .get(&url)
        .bearer_auth(token)
        .query(&[("fields", "attendees")])
        .send()
        .await
        .map_err(|e| FetchError::Network(e.without_url().to_string()))?;
    if !response.status().is_success() {
        return Err(status_error(response.status()));
    }
    let mut event: serde_json::Value = response
        .json()
        .await
        .map_err(|e| FetchError::Other(e.without_url().to_string()))?;
    let attendees =
        set_my_answer(&mut event, answer).ok_or_else(|| FetchError::Other("not an invite".into()))?;
    let response = http
        .patch(&url)
        .bearer_auth(token)
        .query(&[("sendUpdates", "all")])
        .json(&serde_json::json!({ "attendees": attendees }))
        .send()
        .await
        .map_err(|e| FetchError::Network(e.without_url().to_string()))?;
    if response.status().is_success() {
        Ok(())
    } else {
        Err(status_error(response.status()))
    }
}

/// The attendee list with your answer set, or none if you aren't on it.
pub fn set_my_answer(event: &mut serde_json::Value, answer: InviteResponse) -> Option<serde_json::Value> {
    let attendees = event["attendees"].as_array_mut()?;
    let me = attendees.iter_mut().find(|a| a["self"].as_bool() == Some(true))?;
    me["responseStatus"] = answer.api().into();
    Some(serde_json::Value::Array(attendees.clone()))
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
    fn reads_my_invite_answer_and_changes_it() {
        let json = r#"{"items":[
            {"id":"a","summary":"Sync","start":{"dateTime":"2026-10-07T10:00:00Z"},"end":{"dateTime":"2026-10-07T10:30:00Z"},
             "attendees":[{"email":"boss@x.com","organizer":true,"responseStatus":"accepted"},{"email":"me@x.com","self":true,"responseStatus":"needsAction"}]},
            {"id":"b","summary":"Mine","start":{"dateTime":"2026-10-07T11:00:00Z"},"end":{"dateTime":"2026-10-07T12:00:00Z"},
             "attendees":[{"email":"me@x.com","self":true,"organizer":true,"responseStatus":"accepted"}]}
        ]}"#;
        let events = parse_events(json).unwrap();
        assert_eq!(events[0].my_response, Some(InviteResponse::NeedsAction));
        assert_eq!(events[1].my_response, None);

        let mut event = serde_json::json!({"attendees":[{"email":"boss@x.com"},{"email":"me@x.com","self":true,"responseStatus":"needsAction"}]});
        let list = set_my_answer(&mut event, InviteResponse::Declined).unwrap();
        assert_eq!(list[1]["responseStatus"], "declined");
        assert_eq!(list[0].get("responseStatus"), None);
        assert!(set_my_answer(&mut serde_json::json!({}), InviteResponse::Accepted).is_none());
    }

    #[test]
    fn builds_new_events() {
        let start: DateTime<Utc> = "2026-10-07T10:00:00Z".parse().unwrap();
        let event = NewEvent {
            title: "  Dentist ".into(),
            start,
            end: start + chrono::Duration::minutes(30),
            all_day: false,
            location: Some(" ".into()),
        };
        assert!(event.validate().is_ok());
        let body = new_event_body(&event);
        assert_eq!(body["summary"], "Dentist");
        assert!(body["start"]["dateTime"]
            .as_str()
            .unwrap()
            .starts_with("2026-10-07T10:00:00"));
        assert!(body.get("location").is_none());

        let all_day = NewEvent {
            all_day: true,
            end: start + chrono::Duration::hours(1),
            ..event.clone()
        };
        let body = new_event_body(&all_day);
        let first = body["start"]["date"].as_str().unwrap().to_owned();
        let after: NaiveDate = body["end"]["date"].as_str().unwrap().parse().unwrap();
        assert_eq!(after, first.parse::<NaiveDate>().unwrap().succ_opt().unwrap());

        assert!(NewEvent {
            title: " ".into(),
            ..event.clone()
        }
        .validate()
        .is_err());
        assert!(NewEvent {
            end: start,
            ..event.clone()
        }
        .validate()
        .is_err());
        assert!(NewEvent {
            end: start + chrono::Duration::days(40),
            ..event
        }
        .validate()
        .is_err());
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
