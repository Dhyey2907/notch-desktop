use rusqlite::Connection;

use crate::error::{AppError, AppResult};

/// Ordered, append-only list of migrations. Migration N sets `user_version = N`.
/// Never edit a shipped migration; add a new one.
const MIGRATIONS: &[&str] = &[
    include_str!("migrations/001_init.sql"),
    include_str!("migrations/002_claude_usage.sql"),
    include_str!("migrations/003_approval_audit.sql"),
    include_str!("migrations/004_todo.sql"),
    include_str!("migrations/005_removed_phone_devices.sql"),
    include_str!("migrations/006_drop_phone_devices.sql"),
    include_str!("migrations/007_clip_pins.sql"),
    include_str!("migrations/008_shelf.sql"),
    include_str!("migrations/009_screen_time.sql"),
    include_str!("migrations/010_focus.sql"),
    include_str!("migrations/011_totp.sql"),
    include_str!("migrations/012_coding.sql"),
    include_str!("migrations/013_mail_snooze_read_later.sql"),
];

pub fn schema_version(conn: &Connection) -> AppResult<u32> {
    Ok(conn.pragma_query_value(None, "user_version", |row| row.get(0))?)
}

pub fn migrate(conn: &mut Connection) -> AppResult<()> {
    let current = schema_version(conn)? as usize;
    if current > MIGRATIONS.len() {
        return Err(AppError::Migration(format!(
            "database schema v{current} is newer than this app (v{})",
            MIGRATIONS.len()
        )));
    }

    for (index, sql) in MIGRATIONS.iter().enumerate().skip(current) {
        let version = index + 1;
        let tx = conn.transaction()?;
        tx.execute_batch(sql)
            .map_err(|e| AppError::Migration(format!("migration {version} failed: {e}")))?;
        tx.pragma_update(None, "user_version", version as u32)?;
        tx.commit()?;
        tracing::info!(version, "applied database migration");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migrates_empty_database_to_latest() {
        let mut conn = Connection::open_in_memory().unwrap();
        migrate(&mut conn).unwrap();
        assert_eq!(schema_version(&conn).unwrap() as usize, MIGRATIONS.len());
        conn.execute(
            "INSERT INTO settings (key, value, updated_at) VALUES ('a', '1', 'now')",
            [],
        )
        .unwrap();
    }

    #[test]
    fn rerunning_migrations_is_a_no_op() {
        let mut conn = Connection::open_in_memory().unwrap();
        migrate(&mut conn).unwrap();
        migrate(&mut conn).unwrap();
        assert_eq!(schema_version(&conn).unwrap() as usize, MIGRATIONS.len());
    }

    #[test]
    fn refuses_newer_schema() {
        let mut conn = Connection::open_in_memory().unwrap();
        conn.pragma_update(None, "user_version", 999).unwrap();
        assert!(matches!(migrate(&mut conn), Err(AppError::Migration(_))));
    }
}
