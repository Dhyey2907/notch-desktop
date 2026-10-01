//! Screen time: minutes you actively use the PC, per hour, kept locally in SQLite. A minute
//! counts when the keyboard or mouse was used in the last few minutes (or media is playing)
//! and the lock screen isn't up.

use chrono::{DateTime, Local, Timelike};
use rusqlite::{params, Connection};

use super::ScreenTime;
use crate::error::AppResult;

/// No input for this long means you've stepped away (unless media is playing).
pub const IDLE_AFTER_MS: u64 = 5 * 60 * 1000;
/// Days of history kept.
const KEEP_DAYS: i64 = 30;

/// Whether this minute counts as screen time.
pub fn counts(idle_ms: u64, locked: bool, media_playing: bool) -> bool {
    !locked && (idle_ms < IDLE_AFTER_MS || media_playing)
}

pub fn add_minute(conn: &Connection, at: DateTime<Local>) -> AppResult<()> {
    let day = at.format("%Y-%m-%d").to_string();
    conn.execute(
        "INSERT INTO screen_time (day, hour, seconds) VALUES (?1, ?2, 60)
         ON CONFLICT(day, hour) DO UPDATE SET seconds = MIN(seconds + 60, 3600)",
        params![day, at.hour()],
    )?;
    let oldest = (at - chrono::Duration::days(KEEP_DAYS))
        .format("%Y-%m-%d")
        .to_string();
    conn.execute("DELETE FROM screen_time WHERE day < ?1", [oldest])?;
    Ok(())
}

pub fn today(conn: &Connection, now: DateTime<Local>) -> AppResult<ScreenTime> {
    let day = now.format("%Y-%m-%d").to_string();
    let mut hours = vec![0u32; 24];
    let mut stmt = conn.prepare("SELECT hour, seconds FROM screen_time WHERE day = ?1")?;
    let rows = stmt.query_map([day], |r| Ok((r.get::<_, u32>(0)?, r.get::<_, u32>(1)?)))?;
    for row in rows {
        let (hour, seconds) = row?;
        if let Some(slot) = hours.get_mut(hour as usize) {
            *slot = seconds / 60;
        }
    }
    Ok(ScreenTime {
        minutes: hours.iter().sum(),
        hours,
    })
}

#[cfg(windows)]
pub mod probe {
    //! What the PC is doing right now.
    use windows::Win32::{
        System::{
            SystemInformation::GetTickCount,
            Threading::{
                OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32,
                PROCESS_QUERY_LIMITED_INFORMATION,
            },
        },
        UI::{
            Input::KeyboardAndMouse::{GetLastInputInfo, LASTINPUTINFO},
            WindowsAndMessaging::{GetForegroundWindow, GetWindowThreadProcessId},
        },
    };

    /// Milliseconds since the last keyboard or mouse input.
    pub fn idle_ms() -> u64 {
        let mut info = LASTINPUTINFO {
            cbSize: std::mem::size_of::<LASTINPUTINFO>() as u32,
            dwTime: 0,
        };
        // SAFETY: plain queries filling a local struct.
        unsafe {
            if !GetLastInputInfo(&mut info).as_bool() {
                return 0;
            }
            u64::from(GetTickCount().wrapping_sub(info.dwTime))
        }
    }

    /// The lock screen (LockApp.exe) is in front.
    pub fn locked() -> bool {
        // SAFETY: read-only queries; the process handle is closed before returning.
        unsafe {
            let mut pid = 0;
            GetWindowThreadProcessId(GetForegroundWindow(), Some(&mut pid));
            if pid == 0 {
                return false;
            }
            let Ok(process) = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) else {
                return false;
            };
            let mut buf = [0u16; 512];
            let mut len = buf.len() as u32;
            let ok = QueryFullProcessImageNameW(
                process,
                PROCESS_NAME_WIN32,
                windows::core::PWSTR(buf.as_mut_ptr()),
                &mut len,
            )
            .is_ok();
            let _ = windows::Win32::Foundation::CloseHandle(process);
            ok && String::from_utf16_lossy(&buf[..len as usize])
                .to_lowercase()
                .ends_with("\\lockapp.exe")
        }
    }
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;

    use super::*;

    #[test]
    fn counts_active_minutes_only() {
        assert!(counts(10_000, false, false));
        assert!(!counts(IDLE_AFTER_MS, false, false), "stepped away");
        assert!(counts(IDLE_AFTER_MS * 3, false, true), "watching a video");
        assert!(!counts(0, true, true), "locked");
    }

    #[test]
    fn adds_minutes_per_hour_and_caps_an_hour() {
        let mut conn = Connection::open_in_memory().unwrap();
        crate::db::migrate_for_tests(&mut conn);
        let at = Local.with_ymd_and_hms(2026, 10, 1, 14, 5, 0).unwrap();
        for _ in 0..3 {
            add_minute(&conn, at).unwrap();
        }
        add_minute(&conn, at + chrono::Duration::hours(1)).unwrap();
        for _ in 0..70 {
            add_minute(&conn, at + chrono::Duration::hours(2)).unwrap();
        }
        let today = today(&conn, at).unwrap();
        assert_eq!(today.hours[14], 3);
        assert_eq!(today.hours[15], 1);
        assert_eq!(today.hours[16], 60, "an hour holds at most 60 minutes");
        assert_eq!(today.minutes, 64);
        let tomorrow = super::today(&conn, at + chrono::Duration::days(1)).unwrap();
        assert_eq!(tomorrow.minutes, 0);
    }
}
