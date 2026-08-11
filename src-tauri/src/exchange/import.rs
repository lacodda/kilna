use std::path::Path;

use rusqlite::{Connection, params};
use serde::Serialize;
use serde_json::{Map, Value, json};

use crate::error::{Error, Result};
use crate::release::{self, NewRelease};
use crate::score::{self, NewScore};
use crate::work::version::NewVersion;
use crate::work::{self, NewWork, version};

/// What an import brought in.
#[derive(Debug, Clone, Serialize)]
pub struct ImportReport {
    pub works: usize,
    pub versions: usize,
    pub scores: usize,
    pub releases: usize,
    /// Titles that were already present and were left alone.
    pub skipped: usize,
}

/// Import a slice of a predecessor workspace.
///
/// The source is the author's previous system: a SQLite database with a
/// `songs` table, optional per-song axis snapshots, and a content calendar.
/// Only fields that map onto kilna's model are read; everything specific to
/// that system is left behind, which is the point of the exercise — a model
/// that needs the source's shape to work has not been validated by it.
///
/// Existing works are matched by title and skipped rather than merged: an
/// import that silently rewrites what is already there is not recoverable.
pub fn from_legacy(conn: &mut Connection, source: &Path, profile_id: &str) -> Result<ImportReport> {
    if !source.exists() {
        return Err(Error::Other(format!("no database at {}", source.display())));
    }

    let legacy = Connection::open(source)?;
    ensure_legacy_shape(&legacy)?;

    let existing: Vec<String> = {
        let mut statement = conn.prepare("SELECT title FROM work WHERE profile_id = ?1")?;
        let rows = statement.query_map(params![profile_id], |row| row.get::<_, String>(0))?;
        rows.collect::<rusqlite::Result<Vec<_>>>()?
    };

    // Positional axis snapshots are mapped onto these, in order.
    let axis_keys: Vec<String> = {
        let raw: String = conn.query_row(
            "SELECT config FROM profile WHERE id = ?1",
            params![profile_id],
            |row| row.get(0),
        )?;
        let config: crate::profile::config::ProfileConfig = serde_json::from_str(&raw)?;
        config.axes.into_iter().map(|axis| axis.key).collect()
    };

    let songs = read_songs(&legacy)?;
    let mut report = ImportReport {
        works: 0,
        versions: 0,
        scores: 0,
        releases: 0,
        skipped: 0,
    };

    for song in songs {
        if existing.iter().any(|title| title == &song.title) {
            report.skipped += 1;
            continue;
        }

        let mut meta = Map::new();
        if let Some(bpm) = song.bpm {
            meta.insert("bpm".into(), json!(bpm));
        }
        if let Some(language) = &song.language {
            meta.insert("language".into(), json!(language));
        }

        let created = work::create(
            conn,
            profile_id,
            NewWork {
                kind: "song".into(),
                title: song.title.clone(),
                status: Some(map_status(&song.status)),
                collection_id: None,
                meta: Some(meta),
            },
        )?;
        report.works += 1;

        // The bodies. Lyrics carry the work; style is a parallel role.
        if let Some(lyrics) = song
            .lyrics
            .as_deref()
            .filter(|body| !body.trim().is_empty())
        {
            version::create(
                conn,
                &created.id,
                NewVersion {
                    role: "lyrics".into(),
                    body: lyrics.to_owned(),
                    label: Some("imported".into()),
                    meta: None,
                    make_current: true,
                },
            )?;
            report.versions += 1;
        }
        if let Some(style) = song.style.as_deref().filter(|body| !body.trim().is_empty()) {
            version::create(
                conn,
                &created.id,
                NewVersion {
                    role: "style".into(),
                    body: style.to_owned(),
                    label: Some("imported".into()),
                    meta: None,
                    make_current: false,
                },
            )?;
            report.versions += 1;
        }

        for axes in read_axes(&legacy, &song.id, &axis_keys)? {
            score::create(
                conn,
                &created.id,
                NewScore {
                    axes,
                    version_id: None,
                    note: Some("imported".into()),
                },
            )?;
            report.scores += 1;
        }

        // A song already out becomes a release marked as released; nothing is
        // scheduled by an import, because the source's calendar is its own.
        if song.released {
            let planned = release::create(
                conn,
                NewRelease {
                    work_id: created.id.clone(),
                    kind: "audio".into(),
                    title: Some(song.title.clone()),
                    scheduled_at: None,
                    meta: None,
                },
            )?;
            release::mark_released(conn, &planned.id, None)?;
            report.releases += 1;
        }
    }

    Ok(report)
}

/// Fail early and clearly when handed the wrong database.
fn ensure_legacy_shape(legacy: &Connection) -> Result<()> {
    let songs: i64 = legacy.query_row(
        "SELECT count(*) FROM sqlite_master WHERE type = 'table' AND name = 'songs'",
        [],
        |row| row.get(0),
    )?;

    if songs == 0 {
        return Err(Error::Other(
            "this database has no `songs` table — it is not a workspace kilna can import".into(),
        ));
    }
    Ok(())
}

struct LegacySong {
    id: String,
    title: String,
    status: String,
    bpm: Option<i64>,
    language: Option<String>,
    lyrics: Option<String>,
    style: Option<String>,
    released: bool,
}

fn read_songs(legacy: &Connection) -> Result<Vec<LegacySong>> {
    let mut statement = legacy.prepare(
        "SELECT id, title, status, bpm, language, final_lyrics, final_style, released_at
         FROM songs ORDER BY created_at",
    )?;

    let rows = statement.query_map([], |row| {
        Ok(LegacySong {
            id: row.get(0)?,
            title: row.get(1)?,
            status: row.get(2)?,
            bpm: row.get(3)?,
            language: row.get(4)?,
            lyrics: row.get(5)?,
            style: row.get(6)?,
            released: row.get::<_, Option<i64>>(7)?.is_some(),
        })
    })?;

    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

/// Axis snapshots for one song, oldest first.
///
/// The source stores a snapshot either as an object keyed by axis name or as a
/// bare array of values in the order its axes were defined. The array form is
/// what a real catalogue turned out to contain, so it is mapped onto the
/// profile's axes by position — with the profile's own keys, because a score
/// whose keys nothing recognises is a score that reads as zero.
///
/// Totals are recomputed on insert from the active profile rather than carried
/// over, so an imported score means the same thing as one entered by hand.
fn read_axes(
    legacy: &Connection,
    song_id: &str,
    axis_keys: &[String],
) -> Result<Vec<Map<String, Value>>> {
    let has_table: i64 = legacy.query_row(
        "SELECT count(*) FROM sqlite_master WHERE type = 'table' AND name = 'song_axes'",
        [],
        |row| row.get(0),
    )?;
    if has_table == 0 {
        return Ok(Vec::new());
    }

    let mut statement =
        legacy.prepare("SELECT axes FROM song_axes WHERE song_id = ?1 ORDER BY created_at")?;
    let rows = statement.query_map(params![song_id], |row| row.get::<_, String>(0))?;

    let mut snapshots = Vec::new();
    for raw in rows {
        // A snapshot that will not parse is skipped rather than failing the
        // whole import — one bad row should not cost the other 200 songs.
        let numeric = match serde_json::from_str::<Value>(&raw?) {
            Ok(Value::Object(axes)) => axes
                .into_iter()
                .filter(|(_, value)| value.is_number())
                .collect(),
            // Positional: the nth value belongs to the nth axis of the profile.
            // Extra values are dropped rather than invented into axis names.
            Ok(Value::Array(values)) => values
                .into_iter()
                .zip(axis_keys)
                .filter(|(value, _)| value.is_number())
                .map(|(value, key)| (key.clone(), value))
                .collect(),
            _ => Map::new(),
        };

        if !numeric.is_empty() {
            snapshots.push(numeric);
        }
    }

    Ok(snapshots)
}

/// Map a source status onto the profile's vocabulary.
///
/// Anything unrecognised becomes a draft: an imported work in an unknown state
/// is safest treated as unfinished.
fn map_status(legacy: &str) -> String {
    match legacy {
        "released" => "released",
        "scheduled" | "planned" => "scheduled",
        "evaluated" | "scored" => "scored",
        "shelved" | "archived" | "rejected" => "shelved",
        _ => "draft",
    }
    .to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;
    use crate::profile;
    use crate::work::WorkFilter;

    /// A stand-in for the predecessor's database, with the columns the import
    /// reads. Invented content only.
    fn legacy(path: &Path) {
        let conn = Connection::open(path).unwrap();
        conn.execute_batch(
            "CREATE TABLE songs (
                 id TEXT PRIMARY KEY, title TEXT NOT NULL, status TEXT NOT NULL,
                 bpm INTEGER, language TEXT, final_lyrics TEXT, final_style TEXT,
                 released_at INTEGER, created_at INTEGER NOT NULL
             );
             CREATE TABLE song_axes (
                 id INTEGER PRIMARY KEY, song_id TEXT NOT NULL, axes TEXT NOT NULL,
                 total REAL NOT NULL, created_at INTEGER NOT NULL
             );
             INSERT INTO songs VALUES
                 ('s1', 'Harbour lights', 'released', 96, 'English',
                  'the cranes go still', 'slow indie folk', 1700000000000, 1),
                 ('s2', 'Paper boats', 'draft', 120, 'English', 'we fold the year', NULL, NULL, 2),
                 ('s3', 'Untouched', 'weird-state', NULL, NULL, NULL, NULL, NULL, 3);
             INSERT INTO song_axes VALUES
                 (1, 's1', '{\"hook\": 7, \"lyrics\": 6}', 65.0, 10),
                 (2, 's1', '{\"hook\": 9, \"lyrics\": 8}', 85.0, 20),
                 (3, 's2', 'not json at all', 0.0, 30),
                 -- The positional form a real catalogue turned out to use.
                 (4, 's3', '[9,8,9,7,9,8,9]', 84.0, 40);",
        )
        .unwrap();
    }

    fn workspace() -> (Connection, String) {
        let conn = db::open_in_memory().unwrap();
        profile::seed(&conn).unwrap();
        let profile_id = profile::active(&conn).unwrap().unwrap().id;
        (conn, profile_id)
    }

    #[test]
    fn an_import_brings_works_bodies_and_scores() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("legacy.db");
        legacy(&source);
        let (mut conn, profile_id) = workspace();

        let report = from_legacy(&mut conn, &source, &profile_id).unwrap();

        assert_eq!(report.works, 3);
        assert_eq!(
            report.versions, 3,
            "two bodies for the first, one for the second"
        );
        assert_eq!(
            report.scores, 3,
            "two keyed snapshots and one positional; the unparseable one is skipped"
        );
        assert_eq!(report.releases, 1, "only the one with a release date");
    }

    #[test]
    fn lyrics_become_the_current_version_and_style_stays_parallel() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("legacy.db");
        legacy(&source);
        let (mut conn, profile_id) = workspace();
        from_legacy(&mut conn, &source, &profile_id).unwrap();

        let works = work::list(
            &conn,
            &profile_id,
            &WorkFilter {
                search: Some("Harbour".into()),
                ..Default::default()
            },
        )
        .unwrap();
        let current = version::get(&conn, works[0].current_version_id.as_ref().unwrap())
            .unwrap()
            .unwrap();

        assert_eq!(current.role, "lyrics");
        assert!(
            version::latest(&conn, &works[0].id, "style")
                .unwrap()
                .is_some()
        );
    }

    #[test]
    fn totals_are_recomputed_rather_than_carried_over() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("legacy.db");
        legacy(&source);
        let (mut conn, profile_id) = workspace();
        from_legacy(&mut conn, &source, &profile_id).unwrap();

        let works = work::list(
            &conn,
            &profile_id,
            &WorkFilter {
                search: Some("Harbour".into()),
                ..Default::default()
            },
        )
        .unwrap();
        let history = score::history(&conn, &works[0].id).unwrap();

        // The source recorded 65.0 and 85.0 under its own weights; kilna's
        // Music profile weighs hook and lyrics differently, so an imported
        // score must not simply keep the old number.
        assert_eq!(history.len(), 2);
        assert!(
            (history[0].total - 85.0).abs() > 0.01,
            "the total must come from this profile, not the source"
        );
    }

    #[test]
    fn a_positional_snapshot_is_mapped_onto_the_profiles_axes_in_order() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("legacy.db");
        legacy(&source);
        let (mut conn, profile_id) = workspace();
        from_legacy(&mut conn, &source, &profile_id).unwrap();

        let works = work::list(
            &conn,
            &profile_id,
            &WorkFilter {
                search: Some("Untouched".into()),
                ..Default::default()
            },
        )
        .unwrap();
        let scores = score::history(&conn, &works[0].id).unwrap();

        assert_eq!(scores.len(), 1);
        // The Music profile's first axis is `hook`, and the array led with 9.
        assert_eq!(scores[0].axes.get("hook").unwrap(), &json!(9));
        assert!(
            scores[0].total > 0.0,
            "an unmapped snapshot would read as zero, which is the bug this guards"
        );
    }

    #[test]
    fn an_unknown_status_becomes_a_draft() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("legacy.db");
        legacy(&source);
        let (mut conn, profile_id) = workspace();
        from_legacy(&mut conn, &source, &profile_id).unwrap();

        let works = work::list(
            &conn,
            &profile_id,
            &WorkFilter {
                search: Some("Untouched".into()),
                ..Default::default()
            },
        )
        .unwrap();

        assert_eq!(works[0].status, "draft");
    }

    #[test]
    fn a_second_import_skips_what_is_already_there() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("legacy.db");
        legacy(&source);
        let (mut conn, profile_id) = workspace();
        from_legacy(&mut conn, &source, &profile_id).unwrap();

        let again = from_legacy(&mut conn, &source, &profile_id).unwrap();

        assert_eq!(again.works, 0);
        assert_eq!(again.skipped, 3);
        assert_eq!(
            work::list(&conn, &profile_id, &WorkFilter::default())
                .unwrap()
                .len(),
            3,
            "nothing was duplicated"
        );
    }

    #[test]
    fn importing_the_wrong_database_fails_with_something_readable() {
        let dir = tempfile::tempdir().unwrap();
        let stranger = dir.path().join("other.db");
        Connection::open(&stranger)
            .unwrap()
            .execute_batch("CREATE TABLE unrelated (id INTEGER)")
            .unwrap();
        let (mut conn, profile_id) = workspace();

        let error = from_legacy(&mut conn, &stranger, &profile_id).unwrap_err();

        assert!(error.to_string().contains("songs"), "got {error}");
    }

    #[test]
    fn importing_a_missing_file_fails() {
        let dir = tempfile::tempdir().unwrap();
        let (mut conn, profile_id) = workspace();

        assert!(from_legacy(&mut conn, &dir.path().join("nope.db"), &profile_id).is_err());
    }
}
