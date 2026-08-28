pub mod migrations;

use std::path::{Path, PathBuf};

use rusqlite::Connection;

use crate::error::Result;

/// File name of the workspace database inside the application data directory.
pub const DATABASE_FILE: &str = "kilna.db";

/// Open a workspace, applying any pending migrations.
pub fn open(path: &Path) -> Result<Connection> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let mut conn = Connection::open(path)?;
    configure(&conn)?;
    migrations::apply(&mut conn)?;
    Ok(conn)
}

/// An in-memory workspace — used by tests and by the schema gate in CI.
pub fn open_in_memory() -> Result<Connection> {
    let mut conn = Connection::open_in_memory()?;
    configure(&conn)?;
    migrations::apply(&mut conn)?;
    Ok(conn)
}

/// Pragmas that must be set on every connection, not stored in the file.
fn configure(conn: &Connection) -> Result<()> {
    // WAL survives a crash mid-write and lets a reader run during a write.
    conn.pragma_update(None, "journal_mode", "WAL")?;
    conn.pragma_update(None, "synchronous", "NORMAL")?;
    // Off by default in SQLite; the schema leans on it.
    conn.pragma_update(None, "foreign_keys", true)?;
    Ok(())
}

/// Where the workspace lives on this machine.
pub fn default_path(app_data_dir: &Path) -> PathBuf {
    app_data_dir.join(DATABASE_FILE)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn open_creates_the_file_and_the_schema() {
        let dir = tempfile::tempdir().unwrap();
        let path = default_path(dir.path());

        let conn = open(&path).unwrap();

        assert!(path.exists());
        let tables: i64 = conn
            .query_row(
                "SELECT count(*) FROM sqlite_master WHERE type = 'table' AND name NOT LIKE 'sqlite_%'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(
            tables, 15,
            "nine core tables, three for the assistant, the trash, and two for the focus board"
        );
    }

    #[test]
    fn reopening_a_workspace_keeps_its_data() {
        let dir = tempfile::tempdir().unwrap();
        let path = default_path(dir.path());

        {
            let conn = open(&path).unwrap();
            conn.execute(
                "INSERT INTO profile (id, key, name, config, is_active, is_builtin, created_at, updated_at)
                 VALUES ('p1', 'test', 'Test', '{}', 1, 0, '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')",
                [],
            )
            .unwrap();
        }

        let conn = open(&path).unwrap();
        let count: i64 = conn
            .query_row("SELECT count(*) FROM profile", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn foreign_keys_are_enforced() {
        let conn = open_in_memory().unwrap();

        let result = conn.execute(
            "INSERT INTO work (id, profile_id, kind, title, status, created_at, updated_at)
             VALUES ('w1', 'missing-profile', 'song', 'Title', 'draft', '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')",
            [],
        );

        assert!(result.is_err(), "a work must not outlive its profile");
    }

    #[test]
    fn only_one_profile_can_be_active() {
        let conn = open_in_memory().unwrap();
        let insert = |id: &str, key: &str, active: i64| {
            conn.execute(
                "INSERT INTO profile (id, key, name, config, is_active, is_builtin, created_at, updated_at)
                 VALUES (?1, ?2, ?2, '{}', ?3, 0, '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')",
                rusqlite::params![id, key, active],
            )
        };

        insert("p1", "first", 1).unwrap();
        assert!(insert("p2", "second", 1).is_err());
        insert("p3", "third", 0).unwrap();
    }
}
