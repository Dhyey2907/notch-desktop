//! How much data this PC used: the adapters' byte counters are sampled each minute and the
//! growth is added to today's row. Counters restart from zero after a reboot or an adapter
//! reset, so a smaller reading counts as all-new bytes rather than a negative.

use rusqlite::{params, Connection};
use serde::Serialize;
use ts_rs::TS;

use crate::error::AppResult;

#[derive(Debug, Clone, Default, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct DataUsage {
    /// Bytes downloaded / uploaded today (local time).
    #[ts(type = "number")]
    pub today_down: u64,
    #[ts(type = "number")]
    pub today_up: u64,
    /// This calendar month.
    #[ts(type = "number")]
    pub month_down: u64,
    #[ts(type = "number")]
    pub month_up: u64,
    /// Monthly cap in GB, 0 = none.
    pub cap_gb: u32,
}

/// Growth between two (down, up) counter readings.
pub fn delta(before: (u64, u64), now: (u64, u64)) -> (u64, u64) {
    let grow = |a: u64, b: u64| if b >= a { b - a } else { b };
    (grow(before.0, now.0), grow(before.1, now.1))
}

/// Add bytes to a day's row ("2026-10-16").
pub fn add(conn: &Connection, day: &str, down: u64, up: u64) -> AppResult<()> {
    if down == 0 && up == 0 {
        return Ok(());
    }
    conn.execute(
        "INSERT INTO data_usage (day, down, up) VALUES (?1, ?2, ?3)
         ON CONFLICT(day) DO UPDATE SET down = down + excluded.down, up = up + excluded.up",
        params![day, down as i64, up as i64],
    )?;
    Ok(())
}

/// Today's and this month's totals ("2026-10-16" → month "2026-10").
pub fn read(conn: &Connection, today: &str, cap_gb: u32) -> AppResult<DataUsage> {
    let month = &today[..today.len().min(7)];
    let day: (i64, i64) = conn
        .query_row(
            "SELECT down, up FROM data_usage WHERE day = ?1",
            params![today],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap_or((0, 0));
    let total: (i64, i64) = conn.query_row(
        "SELECT COALESCE(SUM(down), 0), COALESCE(SUM(up), 0) FROM data_usage WHERE day LIKE ?1 || '%'",
        params![month],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )?;
    Ok(DataUsage {
        today_down: day.0.max(0) as u64,
        today_up: day.1.max(0) as u64,
        month_down: total.0.max(0) as u64,
        month_up: total.1.max(0) as u64,
        cap_gb,
    })
}

/// The cap step just crossed (80 or 100 percent), if any, between two month totals.
pub fn crossed(cap_gb: u32, before: u64, now: u64) -> Option<u8> {
    if cap_gb == 0 {
        return None;
    }
    let cap = u64::from(cap_gb) * 1_000_000_000;
    [100u8, 80].into_iter().find(|step| {
        let line = cap * u64::from(*step) / 100;
        before < line && now >= line
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(include_str!("../db/migrations/015_data_usage.sql"))
            .unwrap();
        conn
    }

    #[test]
    fn counts_growth_and_survives_counter_resets() {
        assert_eq!(delta((100, 50), (160, 70)), (60, 20));
        // Rebooted: the counters started again from zero.
        assert_eq!(delta((5_000, 900), (40, 10)), (40, 10));
    }

    #[test]
    fn adds_up_days_into_the_month() {
        let conn = db();
        add(&conn, "2026-09-30", 999, 1).unwrap();
        add(&conn, "2026-10-15", 100, 10).unwrap();
        add(&conn, "2026-10-16", 30, 3).unwrap();
        add(&conn, "2026-10-16", 20, 2).unwrap();
        let u = read(&conn, "2026-10-16", 50).unwrap();
        assert_eq!((u.today_down, u.today_up), (50, 5));
        assert_eq!((u.month_down, u.month_up), (150, 15));
        assert_eq!(u.cap_gb, 50);
    }

    #[test]
    fn warns_once_at_each_step() {
        let gb = 1_000_000_000u64;
        assert_eq!(crossed(10, 7 * gb, 8 * gb), Some(80));
        assert_eq!(crossed(10, 8 * gb, 9 * gb), None);
        assert_eq!(crossed(10, 9 * gb, 10 * gb), Some(100));
        assert_eq!(crossed(0, 0, 99 * gb), None);
    }
}
