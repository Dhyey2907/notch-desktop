//! Text snippets: typing a trigger like `;addr` in any app replaces it with the saved text.
//! Only the last few typed characters are held, in memory, to spot a trigger; they are never
//! stored or logged, and nothing is expanded in password fields.

#[cfg(windows)]
mod hook;
pub mod service;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::error::{AppError, AppResult};

/// Characters kept to spot a trigger (longer than the longest trigger allowed).
const KEEP: usize = 40;
pub const MAX_TRIGGER: usize = 32;
pub const MAX_BODY: usize = 5000;
/// Triggers start with one of these, so ordinary words never expand by accident.
pub const TRIGGER_STARTS: &str = ";:/\\,.!#@$%^&*~`'=+-_";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Snippet {
    pub id: String,
    pub trigger: String,
    pub body: String,
}

pub fn validate(trigger: &str, body: &str) -> AppResult<()> {
    let bad = |m: &str| Err(AppError::InvalidInput(m.into()));
    let count = trigger.chars().count();
    if !(2..=MAX_TRIGGER).contains(&count) {
        return bad("A trigger is 2 to 32 characters, like ;addr");
    }
    if trigger.chars().any(char::is_whitespace) {
        return bad("Triggers can't have spaces");
    }
    if !trigger.starts_with(|c| TRIGGER_STARTS.contains(c)) {
        return bad("Start the trigger with a symbol like ; so normal words don't expand");
    }
    if body.is_empty() || body.chars().count() > MAX_BODY {
        return bad("The text can be up to 5000 characters");
    }
    Ok(())
}

/// The last characters typed in the current field.
#[derive(Default)]
pub struct Typed {
    chars: Vec<char>,
}

impl Typed {
    pub fn push(&mut self, c: char) {
        self.chars.push(c);
        if self.chars.len() > KEEP {
            self.chars.remove(0);
        }
    }

    pub fn backspace(&mut self) {
        self.chars.pop();
    }

    pub fn clear(&mut self) {
        self.chars.clear();
    }

    /// The snippet whose trigger was just typed (the longest one, if several end here).
    pub fn matched<'a>(&self, snippets: &'a [(String, String)]) -> Option<&'a (String, String)> {
        let typed: String = self.chars.iter().collect();
        snippets
            .iter()
            .filter(|(trigger, _)| typed.ends_with(trigger.as_str()))
            .max_by_key(|(trigger, _)| trigger.chars().count())
    }
}

/// Fill in `{date}`, `{time}` and `{datetime}`.
pub fn fill(body: &str, now: chrono::DateTime<chrono::Local>) -> String {
    body.replace("{date}", &now.format("%-d %b %Y").to_string())
        .replace("{time}", &now.format("%H:%M").to_string())
        .replace("{datetime}", &now.format("%-d %b %Y %H:%M").to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spots_the_longest_trigger_just_typed() {
        let snippets = vec![
            (";a".to_owned(), "short".to_owned()),
            (";addr".to_owned(), "12 Main St".to_owned()),
            (";sig".to_owned(), "Thanks".to_owned()),
        ];
        let mut typed = Typed::default();
        for c in "my ;addr".chars() {
            typed.push(c);
        }
        assert_eq!(typed.matched(&snippets).unwrap().1, "12 Main St");
        typed.backspace();
        assert!(typed.matched(&snippets).is_none());
        typed.clear();
        for c in "x;a".chars() {
            typed.push(c);
        }
        assert_eq!(typed.matched(&snippets).unwrap().1, "short");
        for c in "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa;sig".chars() {
            typed.push(c);
        }
        assert_eq!(typed.matched(&snippets).unwrap().1, "Thanks");
    }

    #[test]
    fn checks_triggers() {
        assert!(validate(";addr", "x").is_ok());
        assert!(validate("addr", "x").is_err());
        assert!(validate(";", "x").is_err());
        assert!(validate(";a b", "x").is_err());
        assert!(validate(";ok", "").is_err());
    }

    #[test]
    fn fills_dates() {
        use chrono::TimeZone;
        let at = chrono::Local.with_ymd_and_hms(2026, 10, 1, 9, 5, 0).unwrap();
        assert_eq!(fill("On {date} at {time}", at), "On 1 Oct 2026 at 09:05");
    }
}
