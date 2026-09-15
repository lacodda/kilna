use std::path::{Path, PathBuf};

use rusqlite::Connection;

use crate::error::{Error, Result};

/// Copy the whole workspace to `destination` — the database and its files.
///
/// The database goes through SQLite's own backup API rather than a file
/// copy: with WAL enabled the file on disk is not the whole story, and a
/// plain copy taken mid-write produces a database that opens but is missing
/// recent work.
///
/// The files a person attached go beside it, in a directory named after the
/// backup — `kilna-2026-09-16.db` and `kilna-2026-09-16.media/`. A backup
/// that took only the database would restore a workspace whose every cover
/// pointed at nothing (decision of 2026-09-15), and the paths are inside the
/// workspace, so nothing outside it is copied.
pub fn write(conn: &Connection, destination: &Path, media: Option<&Path>) -> Result<PathBuf> {
    if let Some(parent) = destination.parent() {
        std::fs::create_dir_all(parent)?;
    }

    conn.backup("main", destination, None)?;

    if let Some(media) = media.filter(|dir| dir.is_dir()) {
        let into = media_beside(destination);
        // A second backup to the same name replaces the first, files and
        // all: half of one backup and half of another is not a backup.
        if into.exists() {
            std::fs::remove_dir_all(&into)?;
        }
        copy_dir(media, &into)?;
    }

    Ok(destination.to_path_buf())
}

/// Where a backup keeps the files that belong to it.
fn media_beside(destination: &Path) -> PathBuf {
    destination.with_extension("media")
}

/// Copy a directory, one level of files deep.
///
/// One level is what the workspace has: `media/` holds files named by id and
/// no directories. Walking deeper would be answering a question nobody has
/// asked yet.
fn copy_dir(from: &Path, to: &Path) -> Result<()> {
    std::fs::create_dir_all(to)?;
    for entry in std::fs::read_dir(from)? {
        let entry = entry?;
        if entry.file_type()?.is_file() {
            std::fs::copy(entry.path(), to.join(entry.file_name()))?;
        }
    }
    Ok(())
}

/// Replace the workspace at `target` with the backup at `source`.
///
/// The current workspace is moved aside rather than deleted — a restore is
/// exactly the moment when the thing being replaced turns out to have been
/// wanted after all.
pub fn restore(source: &Path, target: &Path, media: Option<&Path>) -> Result<Option<PathBuf>> {
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

    // And the files that belong to that database. Moved aside rather than
    // merged: a restored workspace whose directory still held the pictures
    // of the one it replaced would show covers that belong to nothing, and
    // the two sets are indistinguishable once mixed.
    if let Some(media) = media {
        let from = media_beside(source);
        if media.exists() {
            let aside = media.with_extension("replaced");
            if aside.exists() {
                std::fs::remove_dir_all(&aside)?;
            }
            std::fs::rename(media, &aside)?;
        }
        if from.is_dir() {
            copy_dir(&from, media)?;
        }
    }

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
                ..NewWork::default()
            },
        )
        .unwrap();
        conn
    }

    /// A backup takes the files with the database, and a restore brings both
    /// back.
    ///
    /// The decision of 2026-09-15: the alternative is a restored workspace
    /// whose every cover points at a file that is not there, which looks
    /// exactly like a workspace whose pictures were lost.
    #[test]
    fn a_backup_carries_the_files_and_a_restore_brings_them_back() {
        let dir = tempfile::tempdir().unwrap();
        let live = dir.path().join("kilna.db");
        let conn = seeded(&live, "Harbour lights");

        let media = dir.path().join("media");
        std::fs::create_dir_all(&media).unwrap();
        std::fs::write(media.join("a.png"), b"a picture").unwrap();
        std::fs::write(media.join("b.png"), b"another").unwrap();

        let copy = dir.path().join("away").join("kilna-backup.db");
        write(&conn, &copy, Some(&media)).unwrap();

        let kept = copy.with_extension("media");
        assert!(kept.is_dir(), "the files travelled with the database");
        assert_eq!(std::fs::read_dir(&kept).unwrap().count(), 2);
        assert_eq!(
            std::fs::read(kept.join("a.png")).unwrap(),
            b"a picture",
            "and they are the same bytes"
        );

        // The workspace moves on: one file replaced, one added.
        std::fs::write(media.join("a.png"), b"changed since").unwrap();
        std::fs::write(media.join("c.png"), b"added since").unwrap();
        drop(conn);

        restore(&copy, &live, Some(&media)).unwrap();

        assert_eq!(
            std::fs::read(media.join("a.png")).unwrap(),
            b"a picture",
            "the restored file is the one the backup held"
        );
        assert!(
            !media.join("c.png").exists(),
            "a file the backup never had does not survive into the restored \
             workspace: its row is not in the restored database either"
        );
        assert!(
            media.with_extension("replaced").join("c.png").exists(),
            "but it is set aside, not destroyed — a restore is exactly when \
             the thing being replaced turns out to have been wanted"
        );
    }

    /// A workspace with no files, and a backup taken of one, are both ordinary.
    #[test]
    fn a_workspace_with_no_files_backs_up_and_restores_the_same_way() {
        let dir = tempfile::tempdir().unwrap();
        let live = dir.path().join("kilna.db");
        let conn = seeded(&live, "Harbour lights");
        let media = dir.path().join("media");

        let copy = dir.path().join("kilna-backup.db");
        write(&conn, &copy, Some(&media)).unwrap();
        assert!(
            !copy.with_extension("media").exists(),
            "nothing to carry, nothing carried"
        );

        drop(conn);
        restore(&copy, &live, Some(&media)).unwrap();
        assert!(live.is_file(), "and the database still restores");
    }
    #[test]
    fn a_backup_contains_the_work_that_was_there() {
        let dir = tempfile::tempdir().unwrap();
        let live = dir.path().join("kilna.db");
        let conn = seeded(&live, "Backed up");
        let copy = dir.path().join("backup.db");

        write(&conn, &copy, None).unwrap();

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
                ..NewWork::default()
            },
        )
        .unwrap();
        let copy = dir.path().join("backup.db");

        write(&conn, &copy, None).unwrap();

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
        write(&conn, &copy, None).unwrap();
        drop(conn);

        let displaced = restore(&copy, &live, None).unwrap();

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

        let result = restore(&stranger, &live, None);

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

        let result = restore(
            &dir.path().join("nothing.db"),
            &dir.path().join("kilna.db"),
            None,
        );

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
