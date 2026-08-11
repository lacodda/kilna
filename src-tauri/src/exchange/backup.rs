use std::path::{Path, PathBuf};

use rusqlite::Connection;

use crate::error::{Error, Result};

/// Copy the whole workspace to `destination`.
///
/// This uses SQLite's own backup API rather than copying the file: with WAL
/// enabled, the file on disk is not the whole story, and a plain copy taken
/// mid-write produces a database that opens but is missing recent work.
pub fn write(conn: &Connection, destination: &Path) -> Result<PathBuf> {
    if let Some(parent) = destination.parent() {
        std::fs::create_dir_all(parent)?;
    }

    conn.backup("main", destination, None)?;
    Ok(destination.to_path_buf())
}

/// Replace the workspace at `target` with the backup at `source`.
///
/// The current workspace is moved aside rather than deleted — a restore is
/// exactly the moment when the thing being replaced turns out to have been
/// wanted after all.
pub fn restore(source: &Path, target: &Path) -> Result<Option<PathBuf>> {
    if !source.exists() {
        return Err(Error::Other(format!("no backup at {}", source.display())));
    }

    // Refuse a file that is not a kilna workspace before touching anything.
    {
        let candidate = Connection::open(source)?;
        let tables: i64 = candidate.query_row(
            "SELECT count(*) FROM sqlite_master WHERE type = 'table' AND name = 'profile'",
            [],
            |row| row.get(0),
        )?;
        if tables == 0 {
            return Err(Error::Other(format!(
                "{} is not a kilna workspace",
                source.display()
            )));
        }
    }

    let displaced = if target.exists() {
        let aside = target.with_extension("db.replaced");
        std::fs::rename(target, &aside)?;
        Some(aside)
    } else {
        None
    };

    // WAL and shared-memory files belong to the workspace being replaced.
    for suffix in ["db-wal", "db-shm"] {
        let stray = target.with_extension(suffix);
        if stray.exists() {
            let _ = std::fs::remove_file(stray);
        }
    }

    std::fs::copy(source, target)?;
    Ok(displaced)
}

/// A dated file name, so successive backups do not overwrite one another.
pub fn suggested_name(timestamp: &str) -> String {
    // The timestamp is RFC 3339; colons are not valid in a Windows file name.
    let safe: String = timestamp
        .chars()
        .map(|c| if c == ':' { '-' } else { c })
        .take(19)
        .collect();
    format!("kilna-{safe}.db")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;
    use crate::profile;
    use crate::work::{self, NewWork, WorkFilter};

    fn seeded(path: &Path, title: &str) -> Connection {
        let conn = db::open(path).unwrap();
        profile::seed(&conn).unwrap();
        let profile_id = profile::active(&conn).unwrap().unwrap().id;
        work::create(
            &conn,
            &profile_id,
            NewWork {
                kind: "song".into(),
                title: title.into(),
                status: None,
                collection_id: None,
                meta: None,
            },
        )
        .unwrap();
        conn
    }

    #[test]
    fn a_backup_contains_the_work_that_was_there() {
        let dir = tempfile::tempdir().unwrap();
        let live = dir.path().join("kilna.db");
        let conn = seeded(&live, "Backed up");
        let copy = dir.path().join("backup.db");

        write(&conn, &copy).unwrap();

        let restored = db::open(&copy).unwrap();
        let profile_id = profile::active(&restored).unwrap().unwrap().id;
        let works = work::list(&restored, &profile_id, &WorkFilter::default()).unwrap();
        assert_eq!(works.len(), 1);
        assert_eq!(works[0].title, "Backed up");
    }

    #[test]
    fn a_backup_taken_with_unflushed_writes_is_still_complete() {
        let dir = tempfile::tempdir().unwrap();
        let live = dir.path().join("kilna.db");
        let conn = seeded(&live, "First");
        let profile_id = profile::active(&conn).unwrap().unwrap().id;
        // With WAL on, this write is not in the main file yet.
        work::create(
            &conn,
            &profile_id,
            NewWork {
                kind: "song".into(),
                title: "Written just now".into(),
                status: None,
                collection_id: None,
                meta: None,
            },
        )
        .unwrap();
        let copy = dir.path().join("backup.db");

        write(&conn, &copy).unwrap();

        let restored = db::open(&copy).unwrap();
        let works = work::list(&restored, &profile_id, &WorkFilter::default()).unwrap();
        assert_eq!(works.len(), 2, "a plain file copy would have missed one");
    }

    #[test]
    fn restore_moves_the_current_workspace_aside() {
        let dir = tempfile::tempdir().unwrap();
        let live = dir.path().join("kilna.db");
        let conn = seeded(&live, "Original");
        let copy = dir.path().join("backup.db");
        write(&conn, &copy).unwrap();
        drop(conn);

        let displaced = restore(&copy, &live).unwrap();

        assert!(displaced.is_some(), "the replaced workspace must be kept");
        assert!(displaced.unwrap().exists());
    }

    #[test]
    fn restore_refuses_a_file_that_is_not_a_workspace() {
        let dir = tempfile::tempdir().unwrap();
        let stranger = dir.path().join("holiday.jpg");
        std::fs::write(&stranger, b"not a database").unwrap();
        let live = dir.path().join("kilna.db");
        let conn = seeded(&live, "Precious");
        drop(conn);

        let result = restore(&stranger, &live);

        assert!(result.is_err());
        // The workspace must be untouched after a refusal.
        let still_there = db::open(&live).unwrap();
        let profile_id = profile::active(&still_there).unwrap().unwrap().id;
        assert_eq!(
            work::list(&still_there, &profile_id, &WorkFilter::default())
                .unwrap()
                .len(),
            1
        );
    }

    #[test]
    fn restore_of_a_missing_backup_fails() {
        let dir = tempfile::tempdir().unwrap();

        let result = restore(&dir.path().join("nothing.db"), &dir.path().join("kilna.db"));

        assert!(result.is_err());
    }

    #[test]
    fn a_suggested_name_is_a_valid_windows_file_name() {
        let name = suggested_name("2026-08-11T03:24:15.123456Z");

        assert!(!name.contains(':'), "got {name}");
        assert!(name.starts_with("kilna-2026-08-11T03-24-15"));
        assert!(name.ends_with(".db"));
    }
}
