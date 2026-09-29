mod migrations;

use std::{path::Path, sync::Mutex};

use rusqlite::Connection;

use crate::error::AppResult;

pub use migrations::schema_version;

/// Single local SQLite connection guarded by a mutex. The app is a low-traffic
/// desktop utility, so one connection is sufficient (plan.md Rule 2).
pub struct Db(Mutex<Connection>);

impl Db {
    pub fn open(path: &Path) -> AppResult<Self> {
        let conn = Connection::open(path)?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        Self::init(conn)
    }

    fn init(mut conn: Connection) -> AppResult<Self> {
        conn.pragma_update(None, "foreign_keys", "ON")?;
        migrations::migrate(&mut conn)?;
        Ok(Self(Mutex::new(conn)))
    }

    /// Run `f` with mutable access, e.g. to open a transaction.
    pub fn with_mut<T>(&self, f: impl FnOnce(&mut Connection) -> AppResult<T>) -> AppResult<T> {
        let mut conn = self.0.lock().unwrap_or_else(|e| e.into_inner());
        f(&mut conn)
    }

    /// Run `f` with exclusive access to the connection.
    pub fn with<T>(&self, f: impl FnOnce(&Connection) -> AppResult<T>) -> AppResult<T> {
        // A poisoned lock only means a previous holder panicked; the connection itself is still valid.
        let conn = self.0.lock().unwrap_or_else(|e| e.into_inner());
        f(&conn)
    }
}

/// Apply all migrations to a scratch connection (tests only).
#[cfg(test)]
pub fn migrate_for_tests(conn: &mut Connection) {
    migrations::migrate(conn).expect("migrations apply");
}
