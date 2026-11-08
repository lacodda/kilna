use std::path::{Path, PathBuf};

use rusqlite::Connection;

use crate::error::Result;

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
    let written = write_database(conn, destination)?;
    copy_media(destination, media)?;
    Ok(written)
}

/// The database half of [`write`]: the part that needs the connection.
///
/// Split so a caller holding the connection behind a lock can let go of it
/// before the files are copied - a folder of clips takes far longer than the
/// database, and nothing about copying it needs the database.
pub fn write_database(conn: &Connection, destination: &Path) -> Result<PathBuf> {
    if let Some(parent) = destination.parent() {
        std::fs::create_dir_all(parent)?;
    }
    // No progress to report: the window waits for the whole copy either way.
    conn.backup("main", destination, None::<fn(rusqlite::backup::Progress)>)?;
    Ok(destination.to_path_buf())
}

/// The files half of [`write`]: the workspace's `media/` beside the database.
pub fn copy_media(destination: &Path, media: Option<&Path>) -> Result<()> {
    if let Some(media) = media.filter(|dir| dir.is_dir()) {
        let into = media_beside(destination);
        // A second backup to the same name replaces the first, files and
        // all: half of one backup and half of another is not a backup.
        if into.exists() {
            std::fs::remove_dir_all(&into)?;
        }
        copy_dir(media, &into)?;
    }
    Ok(())
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
        let conn = crate::fixtures::workspace_file(path);
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

    /// The steps Settings gives for putting a backup back, one by one, as a
    /// person takes them with kilna closed. Kept here so the words on the screen
    /// are held to something that was done (`data.restoreSteps`).
    fn restore_by_the_steps(backup: &Path, database: &Path, media: &Path) {
        let aside = database.parent().unwrap().join("aside");
        std::fs::create_dir_all(&aside).unwrap();

        // The workspace and its files go somewhere safe, and the two files
        // SQLite keeps beside an open database are deleted: they belong to the
        // file moved aside.
        std::fs::rename(database, aside.join("kilna.db")).unwrap();
        if media.exists() {
            std::fs::rename(media, aside.join("media")).unwrap();
        }
        for suffix in ["-wal", "-shm"] {
            let mut beside = database.as_os_str().to_owned();
            beside.push(suffix);
            let beside = std::path::PathBuf::from(beside);
            if beside.exists() {
                std::fs::remove_file(beside).unwrap();
            }
        }

        // The backup's file under the workspace's name, and the folder beside
        // it, when it has one, as `media`.
        std::fs::copy(backup, database).unwrap();
        let files = backup.with_extension("media");
        if files.is_dir() {
            copy_dir(&files, media).unwrap();
        }
    }

    fn titles(database: &Path) -> Vec<String> {
        let conn = db::open(database).unwrap();
        let profile_id = profile::active(&conn).unwrap().unwrap().id;
        work::list(&conn, &profile_id, &WorkFilter::default())
            .unwrap()
            .into_iter()
            .map(|work| work.title)
            .collect()
    }

    /// A backup takes the files with the database, and the steps bring both
    /// back, whatever was done since.
    ///
    /// The decision of 2026-09-15: the alternative is a restored workspace
    /// whose every cover points at a file that is not there, which looks
    /// exactly like a workspace whose pictures were lost.
    #[test]
    fn a_backup_carries_the_files_and_the_steps_bring_them_back() {
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

        // The workspace moves on: a work added, one file replaced, one added.
        let profile_id = profile::active(&conn).unwrap().unwrap().id;
        work::create(
            &conn,
            &profile_id,
            NewWork {
                kind: "song".into(),
                title: "Written since".into(),
                ..NewWork::default()
            },
        )
        .unwrap();
        std::fs::write(media.join("a.png"), b"changed since").unwrap();
        std::fs::write(media.join("c.png"), b"added since").unwrap();
        drop(conn);

        restore_by_the_steps(&copy, &live, &media);

        assert_eq!(
            titles(&live),
            ["Harbour lights"],
            "the work the backup held"
        );
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
        assert_eq!(
            titles(&dir.path().join("aside").join("kilna.db")).len(),
            2,
            "what was moved aside is still whole, should it turn out to be wanted"
        );
    }

    /// A workspace with no files, and a backup taken of one, are both ordinary:
    /// no folder lies beside it, and the step that names it has nothing to do.
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
        restore_by_the_steps(&copy, &live, &media);
        assert_eq!(titles(&live), ["Harbour lights"]);
    }

    /// A write-ahead log left beside a restored file belongs to another database,
    /// which is why the steps delete it: with it gone the backup reads whole.
    #[test]
    fn the_files_sqlite_keeps_beside_a_database_are_part_of_the_steps() {
        let dir = tempfile::tempdir().unwrap();
        let live = dir.path().join("kilna.db");
        let media = dir.path().join("media");
        let conn = seeded(&live, "Harbour lights");
        let copy = dir.path().join("kilna-backup.db");
        write(&conn, &copy, Some(&media)).unwrap();
        drop(conn);
        // What an unclean ending leaves beside the workspace.
        std::fs::write(dir.path().join("kilna.db-wal"), b"another database's log").unwrap();
        std::fs::write(dir.path().join("kilna.db-shm"), b"and its index").unwrap();

        restore_by_the_steps(&copy, &live, &media);

        assert_eq!(titles(&live), ["Harbour lights"]);
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
    fn a_suggested_name_is_a_valid_windows_file_name() {
        let name = suggested_name("2026-08-11T03:24:15.123456Z");

        assert!(!name.contains(':'), "got {name}");
        assert!(name.starts_with("kilna-2026-08-11T03-24-15"));
        assert!(name.ends_with(".db"));
    }
}
