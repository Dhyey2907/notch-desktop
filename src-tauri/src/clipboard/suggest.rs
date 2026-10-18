//! Spots a date/time ("meeting Thu 4pm", "tomorrow 10:30", "12 Oct 3 pm", "in 20 minutes")
//! or a street address in copied text, so the notch can offer "Remind me", "Add to Calendar"
//! or "Open in Maps". Runs locally on the copy; nothing is sent anywhere.
//!
//! It only fires on a clear signal — am/pm, a day word, a weekday, a month, "at"/"by" before a
//! time, or a numeric date — so ordinary numbers in copied text don't trigger it.

use chrono::{Datelike, Duration, NaiveDate, NaiveDateTime, NaiveTime, Weekday};

/// Copies longer than this aren't scanned (a paragraph, a log, code).
pub const MAX_TEXT: usize = 300;
/// Only times within the next year are suggested.
const MAX_AHEAD_DAYS: i64 = 366;
/// A date without a time reminds at 9 am.
const DEFAULT_HOUR: u32 = 9;
/// "tonight" without a time means 8 pm.
const TONIGHT_HOUR: u32 = 20;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct When {
    pub at: NaiveDateTime,
    /// Only a date was given (no time of day).
    pub all_day: bool,
}

fn month(word: &str) -> Option<u32> {
    let w = word.trim_end_matches('.');
    let months = [
        ["jan", "january"],
        ["feb", "february"],
        ["mar", "march"],
        ["apr", "april"],
        ["may", "may"],
        ["jun", "june"],
        ["jul", "july"],
        ["aug", "august"],
        ["sep", "september"],
        ["oct", "october"],
        ["nov", "november"],
        ["dec", "december"],
    ];
    months
        .iter()
        .position(|[short, long]| w == *short || w == *long || (w == "sept" && *short == "sep"))
        .map(|i| i as u32 + 1)
}

fn weekday(word: &str) -> Option<Weekday> {
    let w = word.trim_end_matches('.');
    Some(match w {
        "mon" | "monday" => Weekday::Mon,
        "tue" | "tues" | "tuesday" => Weekday::Tue,
        "wed" | "weds" | "wednesday" => Weekday::Wed,
        "thu" | "thur" | "thurs" | "thursday" => Weekday::Thu,
        "fri" | "friday" => Weekday::Fri,
        "sat" | "saturday" => Weekday::Sat,
        "sun" | "sunday" => Weekday::Sun,
        _ => return None,
    })
}

/// "3", "21st", "2nd" -> day of month.
fn day_of_month(word: &str) -> Option<u32> {
    let digits = word.trim_end_matches(|c: char| c.is_ascii_alphabetic());
    let suffix = &word[digits.len()..];
    if !matches!(suffix, "" | "st" | "nd" | "rd" | "th") || digits.is_empty() || digits.len() > 2 {
        return None;
    }
    digits.parse().ok().filter(|d| (1..=31).contains(d))
}

/// "4pm", "4:30pm", "10.30", "16:30" (with `meridiem` from the next token if any).
fn time_of(word: &str, next: Option<&str>) -> Option<(NaiveTime, bool, usize)> {
    if word == "noon" {
        return Some((NaiveTime::from_hms_opt(12, 0, 0)?, true, 1));
    }
    if word == "midnight" {
        return Some((NaiveTime::from_hms_opt(23, 59, 0)?, true, 1));
    }
    let (body, mut meridiem) = match word {
        w if w.ends_with("am") || w.ends_with("pm") => (&w[..w.len() - 2], Some(&w[w.len() - 2..])),
        w if w.ends_with("a.m.") || w.ends_with("p.m.") => {
            (&w[..w.len() - 4], Some(&w[w.len() - 4..w.len() - 3]))
        }
        w => (w, None),
    };
    let mut used = 1;
    if meridiem.is_none() {
        if let Some(n) = next {
            let n = n.trim_end_matches('.');
            if matches!(n, "am" | "pm" | "a.m" | "p.m") {
                meridiem = Some(&n[..1]);
                used = 2;
            }
        }
    }
    if body.is_empty() || !body.chars().all(|c| c.is_ascii_digit() || c == ':' || c == '.') {
        return None;
    }
    let (h, m) = match body.split_once([':', '.']) {
        Some((h, m)) if m.len() == 2 => (h.parse::<u32>().ok()?, m.parse::<u32>().ok()?),
        Some(_) => return None,
        None => (body.parse::<u32>().ok()?, 0),
    };
    if h > 23 || m > 59 || body.len() > 5 {
        return None;
    }
    let hour = match meridiem.map(|m| m.starts_with('p')) {
        Some(pm) => {
            if !(1..=12).contains(&h) {
                return None;
            }
            match (h, pm) {
                (12, false) => 0,
                (12, true) => 12,
                (h, true) => h + 12,
                (h, false) => h,
            }
        }
        // "16:30" is a time; a bare "4" isn't (unless something else says so).
        None if body.contains([':', '.']) => h,
        None => return None,
    };
    Some((NaiveTime::from_hms_opt(hour, m, 0)?, meridiem.is_some(), used))
}

fn tokens(text: &str) -> Vec<String> {
    text.to_lowercase()
        .split(|c: char| c.is_whitespace() || ",;()[]\"'!?—–".contains(c))
        .map(|t| t.trim_end_matches(['.', ':']).to_owned())
        .filter(|t| !t.is_empty())
        .collect()
}

/// Find a future date/time in `text`, relative to `now` (local time).
pub fn find_when(text: &str, now: NaiveDateTime) -> Option<When> {
    if text.chars().count() > MAX_TEXT || text.matches(['{', '}', ';', '=']).count() > 2 {
        return None;
    }
    let t = tokens(text);
    let mut date: Option<NaiveDate> = None;
    let mut time: Option<NaiveTime> = None;
    let mut strong = false;
    // The date came from a weekday name: a time already past today means next week.
    let mut from_weekday = false;
    let today = now.date();
    let mut i = 0;
    while i < t.len() {
        let w = t[i].as_str();
        let next = t.get(i + 1).map(String::as_str);
        let prev = i.checked_sub(1).map(|p| t[p].as_str());

        // "in 20 minutes", "in 2 hours", "in 3 days"
        if w == "in" {
            if let (Some(n), Some(unit)) = (next.and_then(|n| n.parse::<i64>().ok()), t.get(i + 2)) {
                let span = match unit.as_str() {
                    "min" | "mins" | "minute" | "minutes" => Some(Duration::minutes(n)),
                    "h" | "hr" | "hrs" | "hour" | "hours" => Some(Duration::hours(n)),
                    "day" | "days" => Some(Duration::days(n)),
                    "week" | "weeks" => Some(Duration::weeks(n)),
                    _ => None,
                };
                if let Some(span) = span.filter(|_| n > 0) {
                    return within(now, now + span, false);
                }
            }
        }
        match w {
            "today" => {
                date = Some(today);
                strong = true;
            }
            "tonight" => {
                date = Some(today);
                time = time.or(NaiveTime::from_hms_opt(TONIGHT_HOUR, 0, 0));
                strong = true;
            }
            "tomorrow" | "tmrw" | "tmr" | "tomorow" => {
                date = Some(today + Duration::days(1));
                strong = true;
            }
            "day" if next == Some("after") && t.get(i + 2).map(String::as_str) == Some("tomorrow") => {
                date = Some(today + Duration::days(2));
                strong = true;
                i += 3;
                continue;
            }
            _ => {}
        }
        if let Some(wd) = weekday(w) {
            // "sat"/"sun" are also words: need a time, or "on/next/this" before them.
            let ambiguous = matches!(w, "sat" | "sun");
            let cue = matches!(prev, Some("on" | "next" | "this" | "every"));
            if !ambiguous || cue || t.iter().any(|x| time_of(x, None).is_some_and(|(_, m, _)| m)) {
                let mut ahead = (wd.num_days_from_monday() + 7 - today.weekday().num_days_from_monday()) % 7;
                if prev == Some("next") && ahead == 0 {
                    ahead = 7;
                }
                date = Some(today + Duration::days(i64::from(ahead)));
                from_weekday = true;
                strong = true;
            }
        }
        // "12 Oct", "12th October 2026"
        if let (Some(d), Some(m)) = (day_of_month(w), next.and_then(month)) {
            let year = t
                .get(i + 2)
                .and_then(|y| y.parse::<i32>().ok())
                .filter(|y| (2000..2100).contains(y));
            date = next_date(today, m, d, year);
            strong |= date.is_some();
            i += if year.is_some() { 3 } else { 2 };
            continue;
        }
        // "Oct 12", "October 12th, 2026"
        if let (Some(m), Some(d)) = (month(w), next.and_then(day_of_month)) {
            let year = t
                .get(i + 2)
                .and_then(|y| y.parse::<i32>().ok())
                .filter(|y| (2000..2100).contains(y));
            date = next_date(today, m, d, year);
            strong |= date.is_some();
            i += if year.is_some() { 3 } else { 2 };
            continue;
        }
        // "2026-10-12", "12/10/2026", "12/10" (day first)
        if let Some(d) = numeric_date(w, today) {
            date = Some(d);
            strong = true;
            i += 1;
            continue;
        }
        if let Some((tm, meridiem, used)) = time_of(w, next) {
            time = Some(tm);
            strong |= meridiem || matches!(prev, Some("at" | "by" | "@" | "from"));
            i += used;
            continue;
        }
        i += 1;
    }
    if !strong || (date.is_none() && time.is_none()) {
        return None;
    }
    let all_day = time.is_none();
    let time = time.or(NaiveTime::from_hms_opt(DEFAULT_HOUR, 0, 0))?;
    let at = match date {
        Some(d) if from_weekday && !all_day && d.and_time(time) <= now => {
            (d + Duration::days(7)).and_time(time)
        }
        Some(d) => d.and_time(time),
        // Just a time: today, or tomorrow if it has passed.
        None => {
            let candidate = today.and_time(time);
            if candidate > now {
                candidate
            } else {
                candidate + Duration::days(1)
            }
        }
    };
    within(now, at, all_day)
}

fn within(now: NaiveDateTime, at: NaiveDateTime, all_day: bool) -> Option<When> {
    // An all-day date of today is still worth remembering.
    let future = at > now || (all_day && at.date() == now.date());
    (future && at < now + Duration::days(MAX_AHEAD_DAYS)).then_some(When { at, all_day })
}

/// The next date with this month and day (this year, or next if it has passed).
fn next_date(today: NaiveDate, month: u32, day: u32, year: Option<i32>) -> Option<NaiveDate> {
    if let Some(y) = year {
        return NaiveDate::from_ymd_opt(y, month, day);
    }
    let this_year = NaiveDate::from_ymd_opt(today.year(), month, day)?;
    if this_year >= today {
        Some(this_year)
    } else {
        NaiveDate::from_ymd_opt(today.year() + 1, month, day)
    }
}

fn numeric_date(word: &str, today: NaiveDate) -> Option<NaiveDate> {
    if let Some((y, rest)) = word.split_once('-') {
        let (m, d) = rest.split_once('-')?;
        if y.len() == 4 {
            return NaiveDate::from_ymd_opt(y.parse().ok()?, m.parse().ok()?, d.parse().ok()?);
        }
        return None;
    }
    let parts: Vec<&str> = word.split('/').collect();
    if !(2..=3).contains(&parts.len())
        || !parts
            .iter()
            .all(|p| !p.is_empty() && p.chars().all(|c| c.is_ascii_digit()))
    {
        return None;
    }
    let (d, m) = (parts[0].parse().ok()?, parts[1].parse().ok()?);
    let year = parts
        .get(2)
        .and_then(|y| y.parse::<i32>().ok())
        .map(|y| if y < 100 { 2000 + y } else { y });
    next_date(today, m, d, year)
}

/// A street address: a postal code or street word, digits, and commas, in a short copy.
pub fn find_address(text: &str) -> Option<String> {
    let one_line = text
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .collect::<Vec<_>>()
        .join(", ");
    let len = one_line.chars().count();
    if !(12..=250).contains(&len) || one_line.contains("://") || one_line.contains('@') {
        return None;
    }
    let lower = one_line.to_lowercase();
    let words: Vec<&str> = lower
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .collect();
    let street = words.iter().any(|w| {
        matches!(
            *w,
            "street"
                | "st"
                | "road"
                | "rd"
                | "avenue"
                | "ave"
                | "lane"
                | "ln"
                | "marg"
                | "nagar"
                | "society"
                | "sector"
                | "apartment"
                | "apartments"
                | "apt"
                | "floor"
                | "block"
                | "colony"
                | "highway"
                | "boulevard"
                | "blvd"
                | "drive"
                | "dr"
                | "plaza"
                | "chowk"
                | "char"
                | "rasta"
                | "near"
                | "opp"
                | "opposite"
                | "complex"
                | "tower"
                | "suite"
        )
    });
    // Indian PIN (6 digits) or US ZIP (5 digits).
    let postal = words
        .iter()
        .any(|w| (w.len() == 6 || w.len() == 5) && w.chars().all(|c| c.is_ascii_digit()));
    let has_digit = one_line.chars().any(|c| c.is_ascii_digit());
    let commas = one_line.matches(',').count();
    (has_digit && commas >= 1 && (street || postal) && (street || commas >= 2)).then_some(one_line)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Thursday 1 Oct 2026, 11:00.
    fn now() -> NaiveDateTime {
        NaiveDate::from_ymd_opt(2026, 10, 1)
            .unwrap()
            .and_hms_opt(11, 0, 0)
            .unwrap()
    }

    fn at(text: &str) -> Option<(String, bool)> {
        find_when(text, now()).map(|w| (w.at.format("%a %d %b %H:%M").to_string(), w.all_day))
    }

    #[test]
    fn reads_common_phrasings() {
        assert_eq!(at("meeting Thu 4pm"), Some(("Thu 01 Oct 16:00".into(), false)));
        assert_eq!(
            at("call mom tomorrow 10:30"),
            Some(("Fri 02 Oct 10:30".into(), false))
        );
        assert_eq!(
            at("Interview on 12 Oct at 3 pm"),
            Some(("Mon 12 Oct 15:00".into(), false))
        );
        assert_eq!(
            at("Dentist Oct 12th, 9.30 am"),
            Some(("Mon 12 Oct 09:30".into(), false))
        );
        assert_eq!(
            at("standup next monday 9:15am"),
            Some(("Mon 05 Oct 09:15".into(), false))
        );
        assert_eq!(at("dinner tonight"), Some(("Thu 01 Oct 20:00".into(), false)));
        assert_eq!(at("pay rent by 5pm"), Some(("Thu 01 Oct 17:00".into(), false)));
        assert_eq!(
            at("submit by 10am"),
            Some(("Fri 02 Oct 10:00".into(), false)),
            "10am today has passed"
        );
        assert_eq!(
            at("party day after tomorrow 7 pm"),
            Some(("Sat 03 Oct 19:00".into(), false))
        );
        assert_eq!(
            at("noon lunch tomorrow"),
            Some(("Fri 02 Oct 12:00".into(), false))
        );
        assert_eq!(
            at("Webinar 2026-11-03 18:00"),
            Some(("Tue 03 Nov 18:00".into(), false))
        );
        assert_eq!(at("Due 15/10"), Some(("Thu 15 Oct 09:00".into(), true)));
        assert_eq!(
            at("Holiday on 25 December"),
            Some(("Fri 25 Dec 09:00".into(), true))
        );
        assert_eq!(
            at("Flight 5 Jan 2027 6:45 a.m."),
            Some(("Tue 05 Jan 06:45".into(), false))
        );
        assert_eq!(at("12 am"), Some(("Fri 02 Oct 00:00".into(), false)));
    }

    #[test]
    fn handles_relative_times() {
        assert_eq!(
            at("remind me in 20 minutes"),
            Some(("Thu 01 Oct 11:20".into(), false))
        );
        assert_eq!(
            at("check back in 2 hours"),
            Some(("Thu 01 Oct 13:00".into(), false))
        );
        assert_eq!(at("in 3 days"), Some(("Sun 04 Oct 11:00".into(), false)));
    }

    #[test]
    fn rolls_past_dates_to_next_year() {
        assert_eq!(at("birthday 3 Feb"), Some(("Wed 03 Feb 09:00".into(), true)));
        assert_eq!(
            at("Thursday"),
            Some(("Thu 01 Oct 09:00".into(), true)),
            "today, all day"
        );
        assert_eq!(at("next thursday 4pm"), Some(("Thu 08 Oct 16:00".into(), false)));
        assert_eq!(
            at("Thu 9am"),
            Some(("Thu 08 Oct 09:00".into(), false)),
            "9am today has passed"
        );
    }

    #[test]
    fn ignores_ordinary_text() {
        for text in [
            "version 4.30 is out",
            "score was 16:30 at half time",
            "I sat down and it was a sunny day",
            "you may want to check this",
            "The quick brown fox",
            "let x = 4; if (a) { b = 2; }",
            "error at line 42",
            "4",
            "",
            "Paid on 2019-01-01",
        ] {
            assert_eq!(at(text), None, "{text:?}");
        }
        assert_eq!(at(&"meeting thu 4pm ".repeat(30)), None, "too long");
    }

    #[test]
    fn spots_addresses() {
        assert_eq!(
            find_address("A-12, Shanti Nagar Society\nNear City Mall, Ahmedabad 380015").as_deref(),
            Some("A-12, Shanti Nagar Society, Near City Mall, Ahmedabad 380015")
        );
        assert!(find_address("221B Baker Street, London").is_some());
        assert!(find_address("1600 Amphitheatre Pkwy, Mountain View, CA 94043").is_some());
        assert!(find_address("Call me at 9876543210").is_none());
        assert!(find_address("https://maps.google.com/?q=Baker+Street, 2").is_none());
        assert!(find_address("Order #12345, shipped").is_none());
        assert!(find_address("Meeting at 4pm, room 2").is_none());
    }
}

/// A parcel tracking number copied on its own: the carrier (a guess from its shape) and a
/// tracking page. Numbers that are probably something else (phone numbers) are left alone.
pub fn find_tracking(text: &str) -> Option<(&'static str, String)> {
    let t: String = text.trim().chars().filter(|c| !c.is_whitespace()).collect();
    if t.len() < 10
        || t.len() > 30
        || !t.chars().all(|c| c.is_ascii_alphanumeric())
        || text.trim().contains('\n')
    {
        return None;
    }
    let up = t.to_ascii_uppercase();
    let digits = up.chars().filter(char::is_ascii_digit).count();
    let all_digits = digits == up.len();
    let carrier = if up.starts_with("1Z") && up.len() == 18 {
        "UPS"
    } else if up.starts_with("TBA") && up.len() == 15 && up[3..].chars().all(|c| c.is_ascii_digit()) {
        "Amazon"
    } else if up.len() == 13
        && up[..2].chars().all(|c| c.is_ascii_alphabetic())
        && up[2..11].chars().all(|c| c.is_ascii_digit())
        && up[11..].chars().all(|c| c.is_ascii_alphabetic())
    {
        // The international postal format: India Post, Royal Mail, USPS international…
        if up.ends_with("IN") {
            "India Post"
        } else {
            "Post"
        }
    } else if all_digits && (up.len() == 20 || up.len() == 22) {
        "USPS"
    } else if all_digits && (up.len() == 15 || (up.len() == 12 && !up.starts_with("91"))) {
        "FedEx"
    } else {
        return None;
    };
    Some((carrier, format!("https://parcelsapp.com/en/tracking/{up}")))
}

#[cfg(test)]
mod tracking_tests {
    use super::find_tracking;

    #[test]
    fn spots_tracking_numbers_and_skips_phone_numbers() {
        assert_eq!(find_tracking("1Z999AA10123456784").map(|t| t.0), Some("UPS"));
        assert_eq!(find_tracking(" EE123456789IN ").map(|t| t.0), Some("India Post"));
        assert_eq!(find_tracking("TBA123456789012").map(|t| t.0), Some("Amazon"));
        assert_eq!(find_tracking("9400111899223197428490").map(|t| t.0), Some("USPS"));
        assert_eq!(find_tracking("123456789012").map(|t| t.0), Some("FedEx"));
        assert!(find_tracking("919876543210").is_none());
        assert!(find_tracking("9876543210").is_none());
        assert!(find_tracking("hello world").is_none());
        assert!(find_tracking("1Z999AA1012345678\nmore").is_none());
        assert!(find_tracking("1Z999AA10123456784")
            .unwrap()
            .1
            .ends_with("/1Z999AA10123456784"));
    }
}
