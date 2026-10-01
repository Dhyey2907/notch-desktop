//! Your day: an AI plan made from your tasks, meetings and deadlines (Claude Code, once a
//! morning or on request), and Google Classroom assignments that are due soon.

pub mod service;

use chrono::{DateTime, Local, NaiveDate, NaiveTime, TimeZone, Utc};
use serde::Serialize;
use serde_json::Value;
use ts_rs::TS;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Assignment {
    pub id: String,
    pub course: String,
    pub title: String,
    pub due: DateTime<Utc>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub link: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct DayPlan {
    /// Markdown-ish lines from Claude.
    pub text: String,
    pub made_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum ClassroomStatus {
    Off,
    NeedsSignIn,
    Ready,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct DayState {
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plan: Option<DayPlan>,
    pub planning: bool,
    pub classroom: ClassroomStatus,
    /// Not turned in, due within the next three weeks (soonest first).
    pub assignments: Vec<Assignment>,
}

/// Classroom's due date (UTC) from `dueDate` + optional `dueTime`. No time means end of day.
pub fn due_of(work: &Value) -> Option<DateTime<Utc>> {
    let d = &work["dueDate"];
    let date = NaiveDate::from_ymd_opt(
        d["year"].as_i64()? as i32,
        d["month"].as_u64()? as u32,
        d["day"].as_u64()? as u32,
    )?;
    let t = &work["dueTime"];
    // Classroom leaves out fields that are zero ({"hours": 9} is 09:00, {} is midnight).
    let time = if t.is_object() {
        NaiveTime::from_hms_opt(
            t["hours"].as_u64().unwrap_or(0) as u32,
            t["minutes"].as_u64().unwrap_or(0) as u32,
            0,
        )?
    } else {
        NaiveTime::from_hms_opt(23, 59, 0)?
    };
    Some(Utc.from_utc_datetime(&date.and_time(time)))
}

/// Remind once when an assignment is due within a day.
pub fn due_soon(due: DateTime<Utc>, now: DateTime<Utc>) -> bool {
    let left = due - now;
    left > chrono::Duration::zero() && left <= chrono::Duration::hours(24)
}

/// What Claude gets to plan the day: only titles and times, nothing else.
pub fn plan_input(
    now: DateTime<Local>,
    tasks: &[String],
    meetings: &[(DateTime<Local>, DateTime<Local>, String)],
    due: &[(DateTime<Local>, String)],
    workday: (NaiveTime, NaiveTime),
) -> String {
    let mut out = format!(
        "Today is {}. It is {} now. Work hours: {} to {}.\n",
        now.format("%A %-d %B %Y"),
        now.format("%H:%M"),
        workday.0.format("%H:%M"),
        workday.1.format("%H:%M")
    );
    out.push_str("\nMeetings today:\n");
    if meetings.is_empty() {
        out.push_str("- none\n");
    }
    for (start, end, title) in meetings {
        out.push_str(&format!(
            "- {}–{} {title}\n",
            start.format("%H:%M"),
            end.format("%H:%M")
        ));
    }
    out.push_str("\nOpen tasks:\n");
    if tasks.is_empty() {
        out.push_str("- none\n");
    }
    for task in tasks.iter().take(30) {
        out.push_str(&format!("- {task}\n"));
    }
    if !due.is_empty() {
        out.push_str("\nDue soon:\n");
        for (when, title) in due.iter().take(10) {
            out.push_str(&format!("- {title} (due {})\n", when.format("%a %H:%M")));
        }
    }
    out
}

pub const PLAN_PROMPT: &str = "You plan someone's day. From their meetings, open tasks and deadlines, \
write a short realistic schedule for the rest of today. One line per block, like \"09:30–10:30 Write report\". \
Keep meetings where they are, put urgent and due-soon work first, add short breaks, don't overfill. \
After the schedule, one line starting with \"Focus:\" naming the single most important thing. \
Plain text only, no headings, no code fences, at most 12 lines.";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_classroom_due_dates() {
        let with_time = serde_json::json!({"dueDate": {"year": 2026, "month": 10, "day": 3}, "dueTime": {"hours": 18, "minutes": 30}});
        assert_eq!(
            due_of(&with_time).unwrap().to_rfc3339(),
            "2026-10-03T18:30:00+00:00"
        );
        let no_time = serde_json::json!({"dueDate": {"year": 2026, "month": 10, "day": 3}});
        assert_eq!(
            due_of(&no_time).unwrap().to_rfc3339(),
            "2026-10-03T23:59:00+00:00"
        );
        // Zero fields are left out.
        let hours =
            serde_json::json!({"dueDate": {"year": 2026, "month": 10, "day": 3}, "dueTime": {"hours": 9}});
        assert_eq!(due_of(&hours).unwrap().to_rfc3339(), "2026-10-03T09:00:00+00:00");
        let midnight = serde_json::json!({"dueDate": {"year": 2026, "month": 10, "day": 3}, "dueTime": {}});
        assert_eq!(
            due_of(&midnight).unwrap().to_rfc3339(),
            "2026-10-03T00:00:00+00:00"
        );
        assert!(due_of(&serde_json::json!({"title": "no due date"})).is_none());
    }

    #[test]
    fn knows_when_something_is_due_soon() {
        let now = Utc.with_ymd_and_hms(2026, 10, 1, 12, 0, 0).unwrap();
        assert!(due_soon(now + chrono::Duration::hours(5), now));
        assert!(!due_soon(now + chrono::Duration::hours(30), now));
        assert!(!due_soon(now - chrono::Duration::hours(1), now));
    }

    #[test]
    fn writes_only_titles_and_times_for_the_planner() {
        let now = Local.with_ymd_and_hms(2026, 10, 1, 9, 0, 0).unwrap();
        let meeting = (
            now + chrono::Duration::hours(2),
            now + chrono::Duration::hours(3),
            "Standup".to_owned(),
        );
        let input = plan_input(
            now,
            &["Write report".into()],
            &[meeting],
            &[],
            (
                NaiveTime::from_hms_opt(9, 0, 0).unwrap(),
                NaiveTime::from_hms_opt(18, 0, 0).unwrap(),
            ),
        );
        assert!(input.contains("Thursday 1 October 2026"));
        assert!(input.contains("- 11:00–12:00 Standup"));
        assert!(input.contains("- Write report"));
        assert!(!input.contains("Due soon"));
    }
}
