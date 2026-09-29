//! Phone notifications, via Microsoft Phone Link.
//!
//! Phone Link has no API, but the notifications it shows land in the Windows Action Center,
//! which apps may read (`UserNotificationListener`) once the user allows it. Windows offers no
//! change event to unpackaged apps, so the service watches the Action Center's database file
//! and re-reads the list only when it changes. Notification text is kept in memory only —
//! never logged or written to disk. Replying happens in Phone Link (there is no API to reply
//! through another app's notification).

pub mod service;

use std::collections::HashSet;

use chrono::{DateTime, Utc};
use serde::Serialize;
use ts_rs::TS;

/// Most recent phone notifications kept for the expanded view.
const KEEP: usize = 50;
const MAX_TEXT: usize = 500;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct PhoneNotification {
    /// Action Center id (used to dismiss it).
    pub id: u32,
    /// Usually the sender or the phone app's title line.
    pub title: String,
    pub body: String,
    pub at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum PhoneAccess {
    /// Notch can read notifications.
    Allowed,
    /// Turned off in Windows Settings → Privacy & security → Notifications.
    Denied,
    /// Not decided yet, or Windows didn't answer.
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct PhoneSnapshot {
    pub access: PhoneAccess,
    pub phone_link_installed: bool,
    /// Phone Link notifications currently in Action Center, newest first.
    pub notifications: Vec<PhoneNotification>,
}

/// A notification as read from Action Center, before filtering.
#[derive(Debug, Clone)]
pub struct RawNotification {
    pub id: u32,
    /// App User Model ID of the app that sent it.
    pub app_id: String,
    pub texts: Vec<String>,
    pub at: DateTime<Utc>,
}

/// Phone Link's package family, which prefixes its notifications' app ids.
const PHONE_LINK_PREFIX: &str = "Microsoft.YourPhone_";

pub fn is_phone_link(app_id: &str) -> bool {
    app_id.starts_with(PHONE_LINK_PREFIX)
}

fn clip(text: &str) -> String {
    let text = text.trim();
    if text.chars().count() > MAX_TEXT {
        text.chars().take(MAX_TEXT).collect::<String>() + "…"
    } else {
        text.to_owned()
    }
}

fn to_phone(raw: &RawNotification) -> PhoneNotification {
    let mut texts = raw.texts.iter().map(|t| t.trim()).filter(|t| !t.is_empty());
    PhoneNotification {
        id: raw.id,
        title: clip(texts.next().unwrap_or("Phone")),
        body: clip(&texts.collect::<Vec<_>>().join("\n")),
        at: raw.at,
    }
}

/// Tracks which notifications have been seen, so only new ones pop up.
#[derive(Debug)]
pub struct Tracker {
    include: fn(&str) -> bool,
    known: HashSet<u32>,
    /// Dismissed in Notch. Windows doesn't let unregistered apps remove other apps'
    /// notifications from Action Center, so dismissing hides them here instead.
    hidden: HashSet<u32>,
    baselined: bool,
    list: Vec<PhoneNotification>,
}

impl Tracker {
    pub fn new(include: fn(&str) -> bool) -> Self {
        Self {
            include,
            known: HashSet::new(),
            hidden: HashSet::new(),
            baselined: false,
            list: Vec::new(),
        }
    }

    pub fn list(&self) -> &[PhoneNotification] {
        &self.list
    }

    /// Hide a notification in Notch. Returns true if it was listed.
    pub fn hide(&mut self, id: u32) -> bool {
        self.hidden.insert(id);
        let before = self.list.len();
        self.list.retain(|n| n.id != id);
        self.list.len() != before
    }

    /// Apply the current Action Center contents. Returns notifications that just arrived
    /// (none on the first call — existing ones are listed but don't pop up) and whether the
    /// list changed.
    pub fn update(&mut self, current: &[RawNotification]) -> (Vec<PhoneNotification>, bool) {
        let mine: Vec<&RawNotification> = current
            .iter()
            .filter(|n| (self.include)(&n.app_id) && !self.hidden.contains(&n.id))
            .collect();
        let ids: HashSet<u32> = mine.iter().map(|n| n.id).collect();
        let arrived: Vec<PhoneNotification> = mine
            .iter()
            .filter(|n| !self.known.contains(&n.id))
            .map(|n| to_phone(n))
            .collect();

        let before = self.list.len();
        self.list.retain(|n| ids.contains(&n.id));
        let removed = self.list.len() != before;
        self.list.extend(arrived.iter().cloned());
        self.list.sort_by_key(|n| std::cmp::Reverse((n.at, n.id)));
        self.list.truncate(KEEP);
        self.known = ids;
        // Forget hidden ids once they're gone from Action Center.
        let present: HashSet<u32> = current.iter().map(|n| n.id).collect();
        self.hidden.retain(|id| present.contains(id));

        let first = !self.baselined;
        self.baselined = true;
        let changed = removed || !arrived.is_empty();
        (if first { Vec::new() } else { arrived }, changed || first)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn raw(id: u32, app: &str, texts: &[&str], secs: i64) -> RawNotification {
        RawNotification {
            id,
            app_id: app.into(),
            texts: texts.iter().map(|t| (*t).to_owned()).collect(),
            at: Utc.timestamp_opt(1_800_000_000 + secs, 0).unwrap(),
        }
    }

    const PL: &str = "Microsoft.YourPhone_8wekyb3d8bbwe!App";

    #[test]
    fn recognises_phone_link() {
        assert!(is_phone_link(PL));
        assert!(!is_phone_link("Microsoft.WindowsStore_8wekyb3d8bbwe!App"));
        assert!(!is_phone_link("{1AC14E77}\\WindowsPowerShell\\powershell.exe"));
    }

    #[test]
    fn existing_notifications_are_listed_but_do_not_pop_up() {
        let mut tracker = Tracker::new(is_phone_link);
        let (arrived, changed) = tracker.update(&[
            raw(1, PL, &["Mom", "Are you coming?"], 0),
            raw(2, "Other.App!App", &["Ignored"], 1),
        ]);
        assert!(arrived.is_empty());
        assert!(changed);
        assert_eq!(tracker.list().len(), 1);
        assert_eq!(
            (tracker.list()[0].title.as_str(), tracker.list()[0].body.as_str()),
            ("Mom", "Are you coming?")
        );
    }

    #[test]
    fn new_ones_pop_up_once_and_cleared_ones_disappear() {
        let mut tracker = Tracker::new(is_phone_link);
        tracker.update(&[raw(1, PL, &["Mom", "Hi"], 0)]);

        let (arrived, changed) = tracker.update(&[
            raw(1, PL, &["Mom", "Hi"], 0),
            raw(3, PL, &["Bank", "OTP 1234", "Do not share"], 5),
        ]);
        assert!(changed);
        assert_eq!(arrived.len(), 1);
        assert_eq!(arrived[0].body, "OTP 1234\nDo not share");
        assert_eq!(tracker.list()[0].id, 3); // newest first

        // Same contents again: nothing new, nothing changed.
        let (arrived, changed) =
            tracker.update(&[raw(1, PL, &["Mom", "Hi"], 0), raw(3, PL, &["Bank", "x"], 5)]);
        assert!(arrived.is_empty() && !changed);

        // Cleared on the phone or in Action Center.
        let (_, changed) = tracker.update(&[raw(3, PL, &["Bank", "x"], 5)]);
        assert!(changed);
        assert_eq!(tracker.list().iter().map(|n| n.id).collect::<Vec<_>>(), [3]);
    }

    #[test]
    fn dismissed_notifications_stay_hidden_until_gone() {
        let mut tracker = Tracker::new(is_phone_link);
        tracker.update(&[raw(1, PL, &["Mom", "Hi"], 0)]);
        assert!(tracker.hide(1));
        assert!(tracker.list().is_empty());
        // Still in Action Center: stays hidden and never pops up again.
        let (arrived, _) = tracker.update(&[raw(1, PL, &["Mom", "Hi"], 0)]);
        assert!(arrived.is_empty() && tracker.list().is_empty());
        // Once Windows drops it, the id is forgotten (ids can be reused).
        tracker.update(&[]);
        assert!(tracker.hidden.is_empty());
        assert!(!tracker.hide(42));
    }

    #[test]
    fn titles_fall_back_and_long_text_is_clipped() {
        let mut tracker = Tracker::new(|_| true);
        tracker.update(&[]);
        let long = "x".repeat(MAX_TEXT + 50);
        let (arrived, _) = tracker.update(&[raw(9, "any", &["  ", &long], 0), raw(10, "any", &[], 1)]);
        let by_id = |id| arrived.iter().find(|n| n.id == id).unwrap();
        assert!(by_id(9).title.ends_with('…') && by_id(9).title.chars().count() == MAX_TEXT + 1);
        assert_eq!(by_id(10).title, "Phone");
    }
}
