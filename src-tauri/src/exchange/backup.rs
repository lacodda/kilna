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

/// The folder of a space its own backups are kept in, beside the database.
pub const KEPT_DIR: &str = "backups";

/// How many of each kind are kept; the oldest beyond that go.
const KEEP_DAILY: usize = 7;
const KEEP_BEFORE_UPGRADE: usize = 3;
const KEEP_BEFORE_RESTORE: usize = 3;

/// Why kilna took a backup by itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
pub enum BackupKind {
    /// The first time the space is opened on a day.
    Daily,
    /// Before a newer kilna changed the database's shape.
    BeforeUpgrade,
    /// Before a backup was put back over it.
    BeforeRestore,
}

impl BackupKind {
    fn prefix(self) -> &'static str {
        match self {
            Self::Daily => "daily-",
            Self::BeforeUpgrade => "before-upgrade-",
            Self::BeforeRestore => "before-restore-",
        }
    }

    fn keeps(self) -> usize {
        match self {
            Self::Daily => KEEP_DAILY,
            Self::BeforeUpgrade => KEEP_BEFORE_UPGRADE,
            Self::BeforeRestore => KEEP_BEFORE_RESTORE,
        }
    }

    const ALL: [Self; 3] = [Self::Daily, Self::BeforeUpgrade, Self::BeforeRestore];
}

/// One backup kilna keeps of a space.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
pub struct KeptBackup {
    pub path: String,
    pub name: String,
    pub kind: BackupKind,
    /// When it was taken, as every stored moment is written.
    pub made_at: String,
    pub bytes: u64,
}

/// A moment as a file name holds it: to the second, without the colons a
/// Windows file name cannot have.
fn stamp(moment: &str) -> String {
    moment
        .chars()
        .map(|c| if c == ':' { '-' } else { c })
        .take(19)
        .collect()
}

/// The moment a stamp was made from.
fn moment(stamp: &str) -> String {
    let (day, time) = stamp.split_at(stamp.len().min(10));
    format!("{day}{}Z", time.replace('-', ":"))
}

/// The backups kilna keeps of the space in `folder`, newest first.
pub fn kept(folder: &Path) -> Result<Vec<KeptBackup>> {
    let dir = folder.join(KEPT_DIR);
    if !dir.is_dir() {
        return Ok(Vec::new());
    }
    let mut found = Vec::new();
    for entry in std::fs::read_dir(&dir)? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().into_owned();
        let Some(stem) = name.strip_suffix(".db") else {
            continue;
        };
        let Some((kind, taken)) = BackupKind::ALL
            .into_iter()
            .find_map(|kind| stem.strip_prefix(kind.prefix()).map(|rest| (kind, rest)))
        else {
            continue;
        };
        // A second copy of one second carries a count after its stamp, and
        // an upgrade's name says the schema it was taken at before it.
        let taken = taken.split('~').next().unwrap_or(taken);
        let taken = &taken[taken.len().saturating_sub(19)..];
        found.push(KeptBackup {
            path: entry.path().display().to_string(),
            made_at: moment(taken),
            name: name.clone(),
            kind,
            bytes: entry.metadata().map_or(0, |meta| meta.len()),
        });
    }
    found.sort_by(|a, b| (&b.made_at, &b.name).cmp(&(&a.made_at, &a.name)));
    Ok(found)
}

/// A name in the kept folder that no file has yet: `stem` itself, or `stem`
/// with a count after it.
///
/// A kept copy is never written over. Two taken within one second would
/// share a name - and a restore from the copy an earlier restore kept, a
/// second later, would have written the present state over the very file it
/// was about to read.
fn free_path(folder: &Path, stem: &str) -> PathBuf {
    let dir = folder.join(KEPT_DIR);
    let mut path = dir.join(format!("{stem}.db"));
    let mut count = 1;
    while path.exists() {
        count += 1;
        path = dir.join(format!("{stem}~{count}.db"));
    }
    path
}

/// Let go of the oldest backups of each kind beyond what is kept.
fn rotate(folder: &Path) -> Result<()> {
    let all = kept(folder)?;
    for kind in BackupKind::ALL {
        for old in all.iter().filter(|one| one.kind == kind).skip(kind.keeps()) {
            if let Err(cause) = std::fs::remove_file(&old.path) {
                crate::log::warn("backup", &format!("could not remove {}: {cause}", old.name));
            }
        }
    }
    Ok(())
}

/// Take a backup of a space before it is opened, when one is due: before a
/// newer kilna changes the database's shape, and the first time it is opened
/// on a day (v0.114).
///
/// Before the migrations on purpose - that is what it is for. A migration
/// rewrites tables in place, and until now the only copy of what was there
/// was one a person had thought to take. The database alone: the files a
/// space keeps are not changed by an upgrade, and copying a folder of clips
/// on every start would be the reason to switch this off.
///
/// Read through a connection that changes nothing, by SQLite's own backup, so
/// what a write-ahead log still holds is in the copy.
pub fn before_open(folder: &Path, database: &Path, now: &str) -> Result<Option<PathBuf>> {
    if !database.is_file() {
        return Ok(None);
    }
    let source = Connection::open_with_flags(
        database,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_URI,
    )?;
    let schema = crate::db::migrations::current_version(&source)?;
    // Nothing in it yet: a space being made, not one to keep a copy of.
    if schema == 0 {
        return Ok(None);
    }
    let stem = if schema < crate::db::migrations::latest_version() {
        format!(
            "{}v{schema}-{}",
            BackupKind::BeforeUpgrade.prefix(),
            stamp(now)
        )
    } else {
        let today = &now[..now.len().min(10)];
        let taken_today = kept(folder)?
            .iter()
            .any(|one| one.kind == BackupKind::Daily && one.made_at.starts_with(today));
        if taken_today {
            return Ok(None);
        }
        format!("{}{}", BackupKind::Daily.prefix(), stamp(now))
    };
    let written = write_database(&source, &free_path(folder, &stem))?;
    rotate(folder)?;
    Ok(Some(written))
}

/// What a file has to be for kilna to put it back over a space.
///
/// Checked before anything is touched: that it is a database, that it is a
/// kilna workspace, that this build can open it, and that SQLite finds it
/// whole.
pub fn check(backup: &Path) -> Result<()> {
    let said = || backup.display().to_string();
    if !backup.is_file() {
        return Err(Error::refused("backup.notFound").param("path", said()));
    }
    let not_a_workspace = || Error::refused("backup.notAWorkspace").param("path", said());
    let source = Connection::open_with_flags(
        backup,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_URI,
    )
    .map_err(|_| not_a_workspace())?;
    let has_profiles: bool = source
        .query_row(
            "SELECT count(*) FROM sqlite_master WHERE type = 'table' AND name IN ('profile', 'work')",
            [],
            |row| row.get::<_, i64>(0),
        )
        .map_err(|_| not_a_workspace())?
        == 2;
    if !has_profiles {
        return Err(not_a_workspace());
    }
    let found = crate::db::migrations::current_version(&source)?;
    let supported = crate::db::migrations::latest_version();
    if found > supported {
        return Err(Error::SchemaTooNew { found, supported });
    }
    let whole: String = source.query_row("PRAGMA quick_check", [], |row| row.get(0))?;
    if whole != "ok" {
        return Err(Error::refused("backup.damaged")
            .param("path", said())
            .param("cause", whole));
    }
    Ok(())
}

/// Put a backup back over the open space: the database through SQLite's own
/// restore, page by page into the open file, and the files beside a backup
/// that has them.
///
/// No file of the database is moved or deleted, so there is no write-ahead
/// log to leave beside the wrong database - the trap of the steps taken by
/// hand - and a second program holding the space open, an agent's
/// `kilna --mcp`, sees the restored rows rather than a file pulled from under
/// it. What stood before is kept first, as a backup of its own.
///
/// The caller brings the restored database up to date afterwards, as opening
/// a space does: a backup from an older kilna is of an older shape.
pub fn restore(
    conn: &mut Connection,
    folder: &Path,
    media: &Path,
    backup: &Path,
    now: &str,
) -> Result<PathBuf> {
    check(backup)?;
    let aside = write_database(
        conn,
        &free_path(
            folder,
            &format!("{}{}", BackupKind::BeforeRestore.prefix(), stamp(now)),
        ),
    )?;
    conn.restore("main", backup, None::<fn(rusqlite::backup::Progress)>)?;

    // A backup taken by hand carries the space's files beside it; the ones
    // here go aside whole and the backup's take their place. One kilna took
    // by itself has none, and the files stay as they are.
    let files = media_beside(backup);
    if files.is_dir() {
        if media.is_dir() {
            let replaced = folder.join(format!("media.replaced-{}", stamp(now)));
            std::fs::rename(media, replaced)?;
        }
        copy_dir(&files, media)?;
    }
    rotate(folder)?;
    Ok(aside)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;
    use crate::profile;
    use crate::work::{self, NewWork, WorkFilter};

    fn seeded(path: &Path, title: &str) -> Connection {
        let conn = crate::fixtures::workspace_file(path);
        profile::seed(&conn, profile::DEFAULT_KIND).unwrap();
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

#[cfg(test)]
mod kept_tests {
    use super::*;
    use crate::fixtures;

    fn titles(conn: &Connection) -> Vec<String> {
        let mut statement = conn
            .prepare("SELECT title FROM work ORDER BY position")
            .unwrap();
        statement
            .query_map([], |row| row.get(0))
            .unwrap()
            .collect::<rusqlite::Result<Vec<String>>>()
            .unwrap()
    }

    fn day(n: u32) -> String {
        format!("2026-11-{n:02}T08:00:00.000Z")
    }

    /// A copy is kept the first time a space is opened on a day, and before
    /// a newer build changes its shape - and the oldest go.
    #[test]
    fn a_copy_is_kept_once_a_day_and_before_an_upgrade() {
        let (conn, profile_id, dir) = fixtures::workspace_on_disk();
        fixtures::song(&conn, &profile_id, "Harbour lights");
        let database = dir.path().join("kilna.db");

        let first = before_open(dir.path(), &database, &day(1)).unwrap();
        assert!(first.is_some(), "no copy on the first opening of a day");
        assert_eq!(
            before_open(dir.path(), &database, "2026-11-01T19:30:00.000Z").unwrap(),
            None,
            "a second copy on the same day"
        );

        // The copy is the workspace: it opens, and holds the work.
        let copy = Connection::open(first.unwrap()).unwrap();
        assert_eq!(titles(&copy), ["Harbour lights"]);
        drop(copy);

        // Nine more days: seven are kept, the newest ones.
        for n in 2..=10 {
            before_open(dir.path(), &database, &day(n)).unwrap();
        }
        let dailies = kept(dir.path()).unwrap();
        assert_eq!(dailies.len(), KEEP_DAILY);
        assert_eq!(dailies[0].made_at, "2026-11-10T08:00:00Z");
        assert_eq!(dailies[KEEP_DAILY - 1].made_at, "2026-11-04T08:00:00Z");

        // A database of an older shape is copied whatever the day, under a
        // name that says the shape it had.
        let latest = crate::db::migrations::latest_version();
        conn.pragma_update(None, "user_version", latest - 1)
            .unwrap();
        let before = before_open(dir.path(), &database, "2026-11-10T09:00:00.000Z")
            .unwrap()
            .expect("no copy before an upgrade");
        conn.pragma_update(None, "user_version", latest).unwrap();
        let name = before.file_name().unwrap().to_string_lossy().into_owned();
        assert_eq!(
            name,
            format!("before-upgrade-v{}-2026-11-10T09-00-00.db", latest - 1)
        );
        let all = kept(dir.path()).unwrap();
        assert_eq!(all[0].kind, BackupKind::BeforeUpgrade);
        assert_eq!(all[0].made_at, "2026-11-10T09:00:00Z");
    }

    /// A backup put back over the open space: what it held is what is here,
    /// and what stood before is kept beside it.
    #[test]
    fn a_backup_is_put_back_over_the_open_space() {
        let (mut conn, profile_id, dir) = fixtures::workspace_on_disk();
        let media = dir.path().join("media");
        fixtures::song(&conn, &profile_id, "First");
        let database = dir.path().join("kilna.db");
        let backup = before_open(dir.path(), &database, &day(1))
            .unwrap()
            .unwrap();
        fixtures::song(&conn, &profile_id, "Second");
        assert_eq!(titles(&conn), ["First", "Second"]);

        let aside = restore(&mut conn, dir.path(), &media, &backup, &day(2)).unwrap();

        assert_eq!(titles(&conn), ["First"]);
        // And the space goes on working: a write lands.
        fixtures::song(&conn, &profile_id, "Third");
        assert_eq!(titles(&conn), ["First", "Third"]);
        // And a restore within the same second keeps a copy of its own
        // rather than writing over the one before it.
        let again = restore(&mut conn, dir.path(), &media, &aside, &day(2)).unwrap();
        assert_ne!(again, aside);
        assert_eq!(titles(&conn), ["First", "Second"]);
        restore(&mut conn, dir.path(), &media, &again, &day(3)).unwrap();
        assert_eq!(titles(&conn), ["First", "Third"]);
        // What was there before the restore can be had back the same way.
        let before = Connection::open(&aside).unwrap();
        assert_eq!(titles(&before), ["First", "Second"]);
        assert!(
            kept(dir.path())
                .unwrap()
                .iter()
                .any(|one| one.kind == BackupKind::BeforeRestore)
        );
    }

    /// A file that is no workspace is refused before anything is touched.
    #[test]
    fn what_is_no_workspace_is_refused_untouched() {
        let (mut conn, profile_id, dir) = fixtures::workspace_on_disk();
        let media = dir.path().join("media");
        fixtures::song(&conn, &profile_id, "First");

        let code = |cause: Error| cause.reason().key;
        let missing = dir.path().join("nowhere.db");
        assert_eq!(
            code(restore(&mut conn, dir.path(), &media, &missing, &day(1)).unwrap_err()),
            "refusal.backup.notFound"
        );
        let text = fixtures::file(dir.path(), "notes.db");
        assert_eq!(
            code(restore(&mut conn, dir.path(), &media, &text, &day(1)).unwrap_err()),
            "refusal.backup.notAWorkspace"
        );
        // Another database that is not kilna's.
        let other = dir.path().join("other.db");
        Connection::open(&other)
            .unwrap()
            .execute_batch("CREATE TABLE thing (id TEXT)")
            .unwrap();
        assert_eq!(
            code(restore(&mut conn, dir.path(), &media, &other, &day(1)).unwrap_err()),
            "refusal.backup.notAWorkspace"
        );

        assert_eq!(titles(&conn), ["First"]);
        assert!(
            kept(dir.path()).unwrap().is_empty(),
            "something was kept aside for a refusal"
        );
    }
}
