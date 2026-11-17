//! Looking after a space: how much room it takes, giving back what it no
//! longer uses, and an examination that says what is wrong with it (v0.114).
//!
//! All of it reads, except [`compact`], which rewrites the database file
//! smaller and changes no row. The examination repairs nothing: what it
//! finds is said, with names, and what to do about a lost file or a stray
//! one is the person's to decide - a doctor that deleted what it took for
//! litter would be one more way to lose a picture.

use std::collections::BTreeSet;
use std::path::Path;

use rusqlite::{Connection, params};
use serde::Serialize;

use crate::error::Result;
use crate::exchange::backup;

/// How much room a space takes on disk, by what takes it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
pub struct Storage {
    /// The database file, with the write-ahead log beside it.
    pub database_bytes: u64,
    /// What compacting would give back: pages the database holds and no
    /// longer uses.
    pub reclaimable_bytes: u64,
    /// The files the space keeps, and how many.
    pub media_bytes: u64,
    pub media_files: usize,
    /// The copies kilna keeps of the database, and how many.
    pub backups_bytes: u64,
    pub backups: usize,
}

fn size_of(path: &Path) -> u64 {
    std::fs::metadata(path).map_or(0, |meta| meta.len())
}

/// The database's own file and the log SQLite keeps beside it.
fn database_size(database: &Path) -> u64 {
    let mut log = database.as_os_str().to_owned();
    log.push("-wal");
    size_of(database) + size_of(Path::new(&log))
}

/// The files of a folder, one level deep, as the space's `media/` is.
fn files_in(dir: &Path) -> Vec<(String, u64)> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    entries
        .filter_map(std::result::Result::ok)
        .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_file()))
        .map(|entry| {
            (
                entry.file_name().to_string_lossy().into_owned(),
                entry.metadata().map_or(0, |meta| meta.len()),
            )
        })
        .collect()
}

/// Measure the space in `folder`, whose database is open on `conn`.
pub fn storage(conn: &Connection, folder: &Path, database: &Path, media: &Path) -> Result<Storage> {
    let free: i64 = conn.query_row("PRAGMA freelist_count", [], |row| row.get(0))?;
    let page: i64 = conn.query_row("PRAGMA page_size", [], |row| row.get(0))?;
    let files = files_in(media);
    let kept = backup::kept(folder)?;
    Ok(Storage {
        database_bytes: database_size(database),
        reclaimable_bytes: u64::try_from(free.saturating_mul(page)).unwrap_or(0),
        media_bytes: files.iter().map(|(_, bytes)| bytes).sum(),
        media_files: files.len(),
        backups_bytes: kept.iter().map(|one| one.bytes).sum(),
        backups: kept.len(),
    })
}

/// What compacting did to the database's size.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
pub struct Compacted {
    pub before_bytes: u64,
    pub after_bytes: u64,
}

/// Rewrite the database without the room it no longer uses.
///
/// A deleted row leaves its page to be used again rather than returning it,
/// so a space that lost a board of pictures' worth of rows keeps their size
/// until this is run. The log is folded in first and after, so the size read
/// is the size on disk.
pub fn compact(conn: &Connection, database: &Path) -> Result<Compacted> {
    let before_bytes = database_size(database);
    conn.execute_batch(
        "PRAGMA wal_checkpoint(TRUNCATE);
         VACUUM;
         PRAGMA wal_checkpoint(TRUNCATE);",
    )?;
    Ok(Compacted {
        before_bytes,
        after_bytes: database_size(database),
    })
}

/// How many names an examination lists of each thing it found; the count
/// says the rest.
const NAMED: usize = 20;

/// Some things of one kind an examination found: how many, and the first
/// of them by name.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
pub struct Found {
    pub count: usize,
    pub names: Vec<String>,
}

impl Found {
    fn of(mut names: Vec<String>) -> Self {
        let count = names.len();
        names.truncate(NAMED);
        Self { count, names }
    }
}

/// What an examination of a space found. Everything empty is a clean bill.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
pub struct Examination {
    /// What SQLite says is wrong with the file itself, in its words.
    pub damage: Vec<String>,
    /// Rows that name a row that is not there.
    pub dangling: usize,
    /// Files attached to nothing: a row with no owner.
    pub ownerless: Found,
    /// Files a row names that are not on disk - the broken pictures.
    pub lost: Found,
    /// Files in the space's folder that nothing names, not even the trash.
    pub stray: Found,
    pub stray_bytes: u64,
}

impl Examination {
    /// Whether anything at all was found.
    pub fn is_clean(&self) -> bool {
        self.damage.is_empty()
            && self.dangling == 0
            && self.ownerless.count == 0
            && self.lost.count == 0
            && self.stray.count == 0
    }
}

/// Examine the space open on `conn`, whose files are kept in `media`.
pub fn examine(conn: &Connection, media: &Path) -> Result<Examination> {
    let damage: Vec<String> = {
        let mut statement = conn.prepare(&format!("PRAGMA integrity_check({NAMED})"))?;
        statement
            .query_map([], |row| row.get::<_, String>(0))?
            .collect::<rusqlite::Result<Vec<_>>>()?
            .into_iter()
            .filter(|line| line != "ok")
            .collect()
    };
    let dangling = {
        let mut statement = conn.prepare("PRAGMA foreign_key_check")?;
        statement.query_map([], |_| Ok(()))?.count()
    };

    // What a row is called when it is said to a person: its label, the name
    // it arrived under, or the name it is stored as.
    let mut statement = conn.prepare(
        "SELECT path, coalesce(nullif(label, ''), original_name, path),
                work_id IS NULL AND release_id IS NULL AND style_brick_id IS NULL
                    AND note_id IS NULL AND trial_id IS NULL
           FROM asset ORDER BY created_at, rowid",
    )?;
    let rows = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, bool>(2)?,
            ))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;

    let mut named: BTreeSet<String> = BTreeSet::new();
    let mut ownerless = Vec::new();
    let mut lost = Vec::new();
    for (path, name, alone) in rows {
        if alone {
            ownerless.push(name.clone());
        }
        if !Path::new(&path).is_file() {
            lost.push(name);
        }
        named.extend(stored_name(&path));
    }
    // What the trash keeps is named too: its files wait with their entries.
    let mut statement = conn.prepare(
        "SELECT json_extract(a.value, '$.path')
           FROM deletion, json_each(deletion.snapshot, '$.asset') AS a",
    )?;
    for path in statement
        .query_map(params![], |row| row.get::<_, Option<String>>(0))?
        .collect::<rusqlite::Result<Vec<_>>>()?
        .into_iter()
        .flatten()
    {
        named.extend(stored_name(&path));
    }

    let mut stray = Vec::new();
    let mut stray_bytes = 0;
    let mut files = files_in(media);
    files.sort();
    for (name, bytes) in files {
        if !named.contains(&name) {
            stray.push(name);
            stray_bytes += bytes;
        }
    }

    Ok(Examination {
        damage,
        dangling,
        ownerless: Found::of(ownerless),
        lost: Found::of(lost),
        stray: Found::of(stray),
        stray_bytes,
    })
}

/// The name a path is stored under in `media/`.
fn stored_name(path: &str) -> Option<String> {
    Path::new(path)
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::asset::{self, NewAsset};
    use crate::fixtures;

    #[test]
    fn a_space_is_measured_by_what_takes_its_room() {
        let (conn, profile_id, dir) = fixtures::workspace_on_disk();
        let media = crate::state::media_dir_of(&conn).unwrap();
        let database = dir.path().join("kilna.db");
        let song = fixtures::song(&conn, &profile_id, "Harbour lights");
        let source = fixtures::file(dir.path(), "take.png");
        asset::attach(
            &conn,
            &profile_id,
            &media,
            &source,
            NewAsset {
                work_id: Some(song.id.clone()),
                ..NewAsset::default()
            },
        )
        .unwrap();
        backup::before_open(dir.path(), &database, "2026-11-01T08:00:00.000Z").unwrap();

        let measured = storage(&conn, dir.path(), &database, &media).unwrap();
        assert!(measured.database_bytes > 0);
        assert_eq!(measured.media_files, 1);
        assert_eq!(measured.media_bytes, size_of(&source));
        assert_eq!(measured.backups, 1);
        assert!(measured.backups_bytes > 0);
    }

    #[test]
    fn compacting_gives_back_what_deleted_rows_held() {
        let (conn, profile_id, dir) = fixtures::workspace_on_disk();
        let media = crate::state::media_dir_of(&conn).unwrap();
        let database = dir.path().join("kilna.db");
        let song = fixtures::song(&conn, &profile_id, "Harbour lights");
        let long = "a line of a song that is long enough to fill a page\n".repeat(400);
        for _ in 0..40 {
            fixtures::version(&conn, &song.id, "lyrics", &long);
        }
        conn.execute("DELETE FROM work_version", []).unwrap();
        let held = storage(&conn, dir.path(), &database, &media).unwrap();
        assert!(
            held.reclaimable_bytes > 0,
            "deleted rows left nothing to give back"
        );

        let compacted = compact(&conn, &database).unwrap();

        assert!(
            compacted.after_bytes < compacted.before_bytes,
            "{compacted:?}"
        );
        assert_eq!(
            storage(&conn, dir.path(), &database, &media)
                .unwrap()
                .reclaimable_bytes,
            0
        );
        // And the rows that were there are there.
        assert!(crate::work::get(&conn, &song.id).unwrap().is_some());
    }

    #[test]
    fn an_examination_names_lost_and_stray_files_and_touches_nothing() {
        let (conn, profile_id, dir) = fixtures::workspace_on_disk();
        let media = crate::state::media_dir_of(&conn).unwrap();
        let song = fixtures::song(&conn, &profile_id, "Harbour lights");
        assert!(examine(&conn, &media).unwrap().is_clean());

        let attach = |name: &str| {
            asset::attach(
                &conn,
                &profile_id,
                &media,
                &fixtures::file(dir.path(), name),
                NewAsset {
                    work_id: Some(song.id.clone()),
                    ..NewAsset::default()
                },
            )
            .unwrap()
        };
        let kept = attach("kept.png");
        let lost = attach("lost.png");
        let trashed = attach("trashed.png");
        // One file gone from under its row, one file nothing names, and one
        // waiting in the trash - which is named, by its entry.
        std::fs::remove_file(&lost.path).unwrap();
        std::fs::write(media.join("stray.bin"), b"12345").unwrap();
        crate::actions::asset::detach(&conn, &trashed.id).unwrap();

        let found = examine(&conn, &media).unwrap();

        assert!(found.damage.is_empty(), "{:?}", found.damage);
        assert_eq!(found.dangling, 0);
        assert_eq!(found.lost, Found::of(vec!["lost.png".into()]));
        assert_eq!(found.stray, Found::of(vec!["stray.bin".into()]));
        assert_eq!(found.stray_bytes, 5);
        assert_eq!(found.ownerless.count, 0);
        // It only looked: the stray file and the kept one are where they were.
        assert!(media.join("stray.bin").is_file());
        assert!(Path::new(&kept.path).is_file());
        assert!(Path::new(&trashed.path).is_file());
    }
}
