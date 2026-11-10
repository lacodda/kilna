pub mod migrations;
pub mod unit;

use std::path::{Path, PathBuf};
use std::sync::{LazyLock, Mutex, PoisonError};
use std::time::Duration;

use rusqlite::Connection;
use rusqlite::backup::Backup;

use crate::error::{Error, Result};

/// File name of a space's database inside its folder.
pub const DATABASE_FILE: &str = "kilna.db";

/// File name of the application's own database inside its data directory:
/// the spaces this machine knows and the settings that are the person's
/// (ADR 0077).
pub const APP_DATABASE_FILE: &str = "app.db";

/// Open the application's database at `path`, applying its own steps.
pub fn open_app(path: &Path) -> Result<Connection> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut conn = Connection::open(path)?;
    configure(&conn)?;
    migrations::apply_app(&mut conn)?;
    Ok(conn)
}

/// The application's database in memory, for tests.
pub fn open_app_in_memory() -> Result<Connection> {
    let mut conn = Connection::open_in_memory()?;
    configure(&conn)?;
    migrations::apply_app(&mut conn)?;
    Ok(conn)
}

/// Open a space's database for reading only - a space held open by another
/// machine (ADR 0077). Nothing is migrated and nothing is seeded: a database
/// this build would have to change is refused rather than opened half-way.
pub fn open_read_only(path: &Path) -> Result<Connection> {
    let conn = Connection::open_with_flags(
        path,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_URI,
    )?;
    conn.pragma_update(None, "foreign_keys", true)?;
    let found = migrations::current_version(&conn)?;
    let supported = migrations::latest_version();
    if found > supported {
        return Err(Error::SchemaTooNew { found, supported });
    }
    if found < supported {
        return Err(Error::refused("space.readOnlyOlder"));
    }
    Ok(conn)
}

/// Open a workspace, applying any pending migrations.
pub fn open(path: &Path) -> Result<Connection> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let mut conn = Connection::open(path)?;
    configure(&conn)?;
    migrations::apply(&mut conn)?;
    // After the migrations, before anything else: the triggers of 0011 name
    // this device on every edit, so it has to exist before the first one.
    crate::device::ensure(&conn)?;
    Ok(conn)
}

/// An in-memory workspace — used by tests and by the schema gate in CI.
///
/// Each one is a page-for-page copy of [`MIGRATED`], so the migrations run
/// once per process rather than once per workspace: in a debug build they
/// cost a third of a second, and most tests open a workspace of their own.
/// The copy carries the schema and `user_version`; the pragmas, which live
/// on the connection, and the device, which must differ between two
/// workspaces, are its own.
pub fn open_in_memory() -> Result<Connection> {
    let mut conn = Connection::open_in_memory()?;
    copy_migrated_into(&mut conn)?;
    configure(&conn)?;
    crate::device::ensure(&conn)?;
    Ok(conn)
}

/// An empty database brought to the latest schema, migrated on first use and
/// never written to after: [`open_in_memory`] only reads it, to copy it.
static MIGRATED: LazyLock<std::result::Result<Mutex<Connection>, String>> = LazyLock::new(|| {
    let migrate = || -> Result<Connection> {
        let mut conn = Connection::open_in_memory()?;
        configure(&conn)?;
        migrations::apply(&mut conn)?;
        Ok(conn)
    };
    migrate().map(Mutex::new).map_err(|error| error.to_string())
});

/// Write a workspace file at `path` holding what the migrations make, copied
/// from [`MIGRATED`] rather than migrated anew - for tests that need a
/// workspace on disk. [`open`] it after, as the window would: the copy has
/// neither the journal mode nor the device a workspace gets on opening.
pub fn write_migrated(path: &Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut file = Connection::open(path)?;
    copy_migrated_into(&mut file)?;
    Ok(())
}

/// Copy every page of [`MIGRATED`] into `into`, replacing what it held.
fn copy_migrated_into(into: &mut Connection) -> Result<()> {
    let migrated = MIGRATED
        .as_ref()
        .map_err(|error| Error::Internal(format!("the migrations failed: {error}")))?
        .lock()
        .unwrap_or_else(PoisonError::into_inner);
    Backup::new(&migrated, into)?.run_to_completion(i32::MAX, Duration::ZERO, None)?;
    Ok(())
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

/// Where the application keeps its data on this platform, without Tauri.
///
/// The same directory Tauri's `app_data_dir` resolves to for this identifier
/// — per-user application data — so `kilna --mcp` opens the workspace the
/// window uses. Resolved by hand because the headless mode has no app handle
/// to ask; the three rules below are the ones Tauri applies.
pub fn default_data_dir() -> Result<PathBuf> {
    const IDENTIFIER: &str = "com.lacodda.kilna";
    let base = if cfg!(target_os = "windows") {
        std::env::var_os("APPDATA").map(PathBuf::from)
    } else if cfg!(target_os = "macos") {
        std::env::var_os("HOME").map(|home| PathBuf::from(home).join("Library/Application Support"))
    } else {
        std::env::var_os("XDG_DATA_HOME")
            .map(PathBuf::from)
            .or_else(|| {
                std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/share"))
            })
    };
    base.map(|dir| dir.join(IDENTIFIER))
        .ok_or_else(|| crate::error::Error::refused("db.noDataDir"))
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
        // The search index is an FTS5 virtual table, and SQLite backs one with
        // half a dozen shadow tables of its own (`search_index_data`, `_idx`,
        // `_docsize`…). They are the index's private business, so they are
        // excluded by name rather than counted: this test is about the tables
        // the product has, not about how FTS5 stores a posting list.
        let tables: i64 = conn
            .query_row(
                "SELECT count(*) FROM sqlite_master \
                  WHERE type = 'table' AND name NOT LIKE 'sqlite_%' \
                    AND name NOT LIKE 'search_index%'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(
            tables, 41,
            "nine core tables, three for the assistant, the trash, two for the focus board, \
             three for the day two workspaces meet (device, tombstone, field_clock), the operations log, \
             the link between works, the scene, what a scene is about, the frames drawn for it, \
             the stretches a short is cut from, the style dictionary, the audience's comments, \
             the canon's facts and relations, the register's terms and the works named for them, \
             the ideas on a cover's board, the shelves of phrases and the notes on them, \
             the repeats the owner chose to keep, where this machine keeps the media, \
             the trials of an experiment, the days the calendar is closed, \
             the keys the search index finds its rows by, \
             the cards on the boards of a work's conception, \
             what was heard in a video's finished file, \
             and what each plugin has been set to - the assistant's settings are the \
             application's since spaces (0050)"
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

    /// Every table, index, trigger and view, with the SQL it was made by.
    fn schema(conn: &Connection) -> Vec<(String, String, Option<String>)> {
        conn.prepare("SELECT type, name, sql FROM sqlite_master ORDER BY type, name")
            .unwrap()
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
            .unwrap()
            .collect::<rusqlite::Result<_>>()
            .unwrap()
    }

    #[test]
    fn an_in_memory_workspace_is_what_the_migrations_make() {
        let mut migrated = Connection::open_in_memory().unwrap();
        configure(&migrated).unwrap();
        migrations::apply(&mut migrated).unwrap();

        let conn = open_in_memory().unwrap();

        assert_eq!(schema(&conn), schema(&migrated));
        assert_eq!(
            migrations::current_version(&conn).unwrap(),
            migrations::latest_version()
        );
        let keys: bool = conn
            .query_row("PRAGMA foreign_keys", [], |row| row.get(0))
            .unwrap();
        assert!(keys, "the schema leans on its foreign keys");
    }

    #[test]
    fn a_workspace_written_from_the_migrated_copy_opens_as_a_new_one() {
        let dir = tempfile::tempdir().unwrap();
        let fresh = open(&dir.path().join("fresh").join(DATABASE_FILE)).unwrap();
        let path = dir.path().join("written").join(DATABASE_FILE);

        write_migrated(&path).unwrap();
        let written = open(&path).unwrap();

        assert_eq!(schema(&written), schema(&fresh));
        let journal = |conn: &Connection| -> String {
            conn.query_row("PRAGMA journal_mode", [], |row| row.get(0))
                .unwrap()
        };
        assert_eq!(journal(&written), "wal");
        assert_eq!(journal(&fresh), "wal");
        assert_eq!(
            migrations::current_version(&written).unwrap(),
            migrations::latest_version()
        );
        let device = |conn: &Connection| crate::device::id(conn).unwrap().unwrap();
        assert_ne!(device(&written), device(&fresh), "each its own device");
    }

    #[test]
    fn two_in_memory_workspaces_share_nothing() {
        let first = open_in_memory().unwrap();
        first
            .execute(
                "INSERT INTO profile (id, key, name, config, is_active, is_builtin, created_at, updated_at)
                 VALUES ('p1', 'test', 'Test', '{}', 1, 0, '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')",
                [],
            )
            .unwrap();

        let second = open_in_memory().unwrap();

        let count: i64 = second
            .query_row("SELECT count(*) FROM profile", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 0, "a workspace opened after another starts empty");
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
