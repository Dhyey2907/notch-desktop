//! Drop shelf: files you drag onto the notch, parked until you drag them out again (or copy
//! them). The shelf keeps only *where* each file is, never a copy, so a file moved or deleted
//! meanwhile shows as missing. File names are never logged.

pub mod service;
#[cfg(windows)]
mod win;
#[cfg(windows)]
pub(crate) use win::{drag_files, reveal};

use std::path::{Path, PathBuf};

use chrono::{DateTime, TimeZone, Utc};
use rusqlite::{params, Connection};
use serde::Serialize;
use ts_rs::TS;

use crate::error::{AppError, AppResult};

pub const MAX_ITEMS: usize = 30;
const MAX_PATH_CHARS: usize = 1024;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ShelfItem {
    pub id: String,
    pub path: String,
    pub name: String,
    /// Lowercase extension without the dot ("pdf"); none for folders.
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ext: Option<String>,
    pub is_dir: bool,
    /// Bytes, for files.
    #[ts(optional, type = "number")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<u64>,
    /// False once the file was moved or deleted.
    pub exists: bool,
    pub added_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ShelfSnapshot {
    /// Most recently added first.
    pub items: Vec<ShelfItem>,
}

/// Only real, absolute paths go on the shelf.
fn check_path(path: &str) -> AppResult<PathBuf> {
    let p = Path::new(path);
    if path.chars().count() > MAX_PATH_CHARS || !p.is_absolute() || path.contains('"') {
        return Err(AppError::InvalidInput("That isn't a file path".into()));
    }
    if !p.exists() {
        return Err(AppError::InvalidInput("That file doesn't exist".into()));
    }
    Ok(p.to_path_buf())
}

/// Put files on the shelf; files already there move to the top. Returns how many were new.
pub fn add(conn: &Connection, paths: &[String], now: DateTime<Utc>) -> AppResult<usize> {
    let paths = paths
        .iter()
        .map(|p| check_path(p))
        .collect::<AppResult<Vec<_>>>()?;
    let mut new = 0;
    let count: i64 = conn.query_row("SELECT COUNT(*) FROM shelf_items", [], |r| r.get(0))?;
    let fresh = paths
        .iter()
        .filter(|p| !exists_in(conn, p).unwrap_or(false))
        .count();
    if count as usize + fresh > MAX_ITEMS {
        return Err(AppError::InvalidInput(format!(
            "The shelf holds {MAX_ITEMS} items — remove some first"
        )));
    }
    // Later files in one drop sort after earlier ones.
    for (i, path) in paths.iter().enumerate() {
        let at = now.timestamp_millis() - i as i64;
        let path = path.display().to_string();
        let updated = conn.execute(
            "UPDATE shelf_items SET added_at = ?2 WHERE path = ?1",
            params![path, at],
        )?;
        if updated == 0 {
            conn.execute(
                "INSERT INTO shelf_items (id, path, added_at) VALUES (?1, ?2, ?3)",
                params![uuid::Uuid::new_v4().to_string(), path, at],
            )?;
            new += 1;
        }
    }
    Ok(new)
}

fn exists_in(conn: &Connection, path: &Path) -> AppResult<bool> {
    let n: i64 = conn.query_row(
        "SELECT COUNT(*) FROM shelf_items WHERE path = ?1",
        [path.display().to_string()],
        |r| r.get(0),
    )?;
    Ok(n > 0)
}

pub fn list(conn: &Connection) -> AppResult<ShelfSnapshot> {
    let mut stmt =
        conn.prepare("SELECT id, path, added_at FROM shelf_items ORDER BY added_at DESC, rowid DESC")?;
    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, i64>(2)?,
        ))
    })?;
    let mut items = Vec::new();
    for row in rows {
        let (id, path, at) = row?;
        items.push(describe(id, path, at));
    }
    Ok(ShelfSnapshot { items })
}

fn describe(id: String, path: String, added_ms: i64) -> ShelfItem {
    let p = Path::new(&path);
    let meta = std::fs::metadata(p).ok();
    let is_dir = meta.as_ref().is_some_and(|m| m.is_dir());
    ShelfItem {
        name: p
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| path.clone()),
        ext: (!is_dir)
            .then(|| p.extension().map(|e| e.to_string_lossy().to_lowercase()))
            .flatten(),
        is_dir,
        size: meta.as_ref().filter(|m| m.is_file()).map(|m| m.len()),
        exists: meta.is_some(),
        added_at: Utc.timestamp_millis_opt(added_ms).single().unwrap_or_default(),
        id,
        path,
    }
}

pub fn remove(conn: &Connection, ids: &[String]) -> AppResult<()> {
    for id in ids {
        conn.execute("DELETE FROM shelf_items WHERE id = ?1", [id])?;
    }
    Ok(())
}

pub fn clear(conn: &Connection) -> AppResult<()> {
    conn.execute("DELETE FROM shelf_items", [])?;
    Ok(())
}

/// Paths of the given items (all items if `ids` is empty) that still exist.
pub fn existing_paths(conn: &Connection, ids: &[String]) -> AppResult<Vec<PathBuf>> {
    let items = list(conn)?.items;
    let paths: Vec<PathBuf> = items
        .into_iter()
        .filter(|i| i.exists && (ids.is_empty() || ids.contains(&i.id)))
        .map(|i| PathBuf::from(i.path))
        .collect();
    if paths.is_empty() {
        return Err(AppError::InvalidInput("Those files were moved or deleted".into()));
    }
    Ok(paths)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn db() -> Connection {
        let mut conn = Connection::open_in_memory().unwrap();
        crate::db::migrate_for_tests(&mut conn);
        conn
    }

    fn temp_files(tag: &str, n: usize) -> (PathBuf, Vec<String>) {
        let dir = std::env::temp_dir().join(format!("notch-shelf-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let files = (0..n)
            .map(|i| {
                let f = dir.join(format!("file {i}.TXT"));
                std::fs::write(&f, "x".repeat(i + 1)).unwrap();
                f.display().to_string()
            })
            .collect();
        (dir, files)
    }

    #[test]
    fn adds_describes_and_dedupes() {
        let conn = db();
        let (dir, files) = temp_files("add", 2);
        assert_eq!(add(&conn, &files, Utc::now()).unwrap(), 2);
        let items = list(&conn).unwrap().items;
        assert_eq!(items[0].name, "file 0.TXT", "earlier files in a drop come first");
        assert_eq!(items[0].ext.as_deref(), Some("txt"));
        assert_eq!(items[1].size, Some(2));
        assert!(items.iter().all(|i| i.exists && !i.is_dir));
        // Dropping one again moves it to the top instead of repeating it.
        assert_eq!(
            add(&conn, &files[1..], Utc::now() + chrono::Duration::seconds(5)).unwrap(),
            0
        );
        let items = list(&conn).unwrap().items;
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].name, "file 1.TXT");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn marks_moved_files_missing() {
        let conn = db();
        let (dir, files) = temp_files("missing", 1);
        add(&conn, &files, Utc::now()).unwrap();
        std::fs::remove_file(&files[0]).unwrap();
        let item = &list(&conn).unwrap().items[0];
        assert!(!item.exists);
        assert!(existing_paths(&conn, &[]).is_err());
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn keeps_folders_and_rejects_bad_paths() {
        let conn = db();
        let (dir, _) = temp_files("dir", 0);
        add(&conn, &[dir.display().to_string()], Utc::now()).unwrap();
        let item = &list(&conn).unwrap().items[0];
        assert!(item.is_dir && item.ext.is_none() && item.size.is_none());
        assert!(add(&conn, &["relative\\file.txt".into()], Utc::now()).is_err());
        assert!(add(&conn, &[dir.join("nope.txt").display().to_string()], Utc::now()).is_err());
        assert!(add(&conn, &["C:\\a\"b".into()], Utc::now()).is_err());
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn holds_a_limited_number_of_items() {
        let conn = db();
        let (dir, files) = temp_files("cap", MAX_ITEMS + 1);
        assert!(add(&conn, &files, Utc::now()).is_err());
        assert!(list(&conn).unwrap().items.is_empty(), "all or nothing");
        add(&conn, &files[..MAX_ITEMS], Utc::now()).unwrap();
        // Re-adding what's already there is fine even when full.
        add(&conn, &files[..1], Utc::now()).unwrap();
        let ids: Vec<String> = list(&conn)
            .unwrap()
            .items
            .iter()
            .take(2)
            .map(|i| i.id.clone())
            .collect();
        remove(&conn, &ids).unwrap();
        assert_eq!(list(&conn).unwrap().items.len(), MAX_ITEMS - 2);
        assert_eq!(existing_paths(&conn, &[]).unwrap().len(), MAX_ITEMS - 2);
        clear(&conn).unwrap();
        assert!(list(&conn).unwrap().items.is_empty());
        let _ = std::fs::remove_dir_all(dir);
    }
}
