//! A door belongs to the work that goes through it.
//!
//! The shipped Studio profile once gave a song three doors: a clip, a short
//! and an audio release. Since a video and a short became works of their own
//! (ADR 0019, ADR 0029) — made from the song, judged on the cut, shipped
//! through doors of their own — the song's clip and short doors said the same
//! thing a second time, on a row that had no relation to the Video or Short
//! work: a `clip` release on a song meant "this song goes out as a video"
//! with nothing to say *which* video, and the key `short` meant two things in
//! one document. From v0.74 a song ships only as `audio`; a clip is a Video
//! work with the song as its donor, shipping through `youtube`; a short is a
//! Short work shipping through `short`. See ADR 0030.
//!
//! This module carries an existing workspace over that line the first time it
//! opens. Every clip or short release hanging on a song or an instrumental is
//! moved onto a new work of the matching kind — one work per release, because
//! each short is its own piece — linked to the song as donor, and the release
//! keeps every date, link and word it had. It is a migration: like the SQL
//! migrations it is not written to the operation log, it is idempotent, and
//! it costs one query when there is nothing to do.

use std::collections::BTreeSet;

use rusqlite::{Connection, params};
use serde_json::{Map, Value};

use crate::error::Result;
use crate::journal::{self, Record};
use crate::link::{self, NewLink};
use crate::minted::Minted;
use crate::profile::config::ProfileConfig;
use crate::release;
use crate::time::now;
use crate::work::{self, NewWork, status};

/// The key of the shipped Studio profile — the only profile this rule is
/// applied to. A profile the owner invented may give a song any door they
/// like; the rule is about the doors kilna itself shipped.
const STUDIO: &str = "music";

/// The kinds of work whose clip and short doors are moved.
const SOURCE_KINDS: [&str; 2] = ["song", "instrumental"];

/// A door that moves: the release kind on the song, the kind of work it
/// becomes, and the door it goes out through from there.
struct Move {
    door: &'static str,
    work_kind: &'static str,
    target_door: &'static str,
}

const MOVES: [Move; 2] = [
    Move {
        door: "clip",
        work_kind: "video",
        target_door: "youtube",
    },
    Move {
        door: "short",
        work_kind: "short",
        target_door: "short",
    },
];

/// Move every clip and short release of a song onto a work of its own, and
/// take the two doors off the song. Returns how many releases moved.
///
/// One transaction per profile: a workspace is either carried over or left
/// as it was, never half of each. Run right after the profiles are seeded,
/// because the carry-forward is what brings the `video` and `short` kinds
/// into a workspace that predates them.
pub fn upgrade(conn: &mut Connection) -> Result<usize> {
    let profiles: Vec<(String, String)> = conn
        .prepare("SELECT id, config FROM profile WHERE key = ?1")?
        .query_map(params![STUDIO], |row| Ok((row.get(0)?, row.get(1)?)))?
        .collect::<rusqlite::Result<_>>()?;

    let mut total = 0;
    for (profile_id, raw) in profiles {
        let mut config: ProfileConfig = serde_json::from_str(&raw)?;
        let tx = conn.transaction()?;

        let moved = move_releases(&tx, &profile_id, &config)?;
        if moved > 0 {
            journal::record(
                &tx,
                &profile_id,
                Record::new("upgrade.doorsMoved").param("count", moved as i64),
            );
        }

        // The doors come off once nothing goes through them any more, which
        // is the moment every release of that door could be moved. A door
        // whose target kind the owner removed keeps its releases and stays.
        if strip_doors(&mut config) {
            tx.execute(
                "UPDATE profile SET config = ?2, updated_at = ?3 WHERE id = ?1",
                params![profile_id, serde_json::to_string(&config)?, now()],
            )?;
        }

        tx.commit()?;
        total += moved;
    }
    Ok(total)
}

/// Whether the kind a door's releases move to can hold a work at all.
///
/// The `video` and `short` kinds reach every workspace through the
/// carry-forward, but an owner may have deleted one since; a release with
/// nowhere to go is left where it is rather than failing the open.
fn target_ready(config: &ProfileConfig, mv: &Move) -> bool {
    config
        .kind(mv.work_kind)
        .is_some_and(|kind| kind.starting_status().is_some())
}

fn move_releases(conn: &Connection, profile_id: &str, config: &ProfileConfig) -> Result<usize> {
    let doors: Vec<&str> = MOVES.iter().map(|mv| mv.door).collect();
    let mut statement = conn.prepare(
        "SELECT r.id, r.kind, r.status, w.id, w.title, w.meta
           FROM release r JOIN work w ON w.id = r.work_id
          WHERE w.profile_id = ?1
            AND w.kind IN (?2, ?3)
            AND r.kind IN (?4, ?5)
          ORDER BY r.created_at, r.rowid",
    )?;
    let rows: Vec<(String, String, String, String, String, String)> = statement
        .query_map(
            params![
                profile_id,
                SOURCE_KINDS[0],
                SOURCE_KINDS[1],
                doors[0],
                doors[1]
            ],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                ))
            },
        )?
        .collect::<rusqlite::Result<_>>()?;

    let mut moved = 0;
    let mut sources = BTreeSet::new();
    for (release_id, door, release_status, source_id, source_title, source_meta) in rows {
        let Some(mv) = MOVES.iter().find(|mv| mv.door == door) else {
            continue;
        };
        if !target_ready(config, mv) {
            continue;
        }

        // The new work takes the song's title and its overview fields, the
        // way *Make a Video from this* does: only fields the profile has, so
        // a stray key in the source's meta is not carried into a new work.
        let meta: Map<String, Value> = serde_json::from_str::<Map<String, Value>>(&source_meta)
            .unwrap_or_default()
            .into_iter()
            .filter(|(key, _)| {
                config
                    .work_meta_fields
                    .iter()
                    .any(|field| field.key == *key)
            })
            .collect();
        let created = work::create_minted(
            conn,
            profile_id,
            NewWork {
                kind: mv.work_kind.to_owned(),
                title: source_title,
                meta: Some(meta),
                ..NewWork::default()
            },
            Minted::fresh(),
        )?;
        link::create_minted(
            conn,
            profile_id,
            NewLink {
                work_id: created.id.clone(),
                source_id: source_id.clone(),
                role: None,
                source_version_id: None,
            },
            Minted::fresh(),
        )?;

        // The release itself only changes hands and door. Its date, its
        // link, its text and its status are the record of what went out, and
        // stay exactly as written.
        conn.execute(
            "UPDATE release SET work_id = ?2, kind = ?3, updated_at = ?4 WHERE id = ?1",
            params![release_id, created.id, mv.target_door, now()],
        )?;

        // A work that shipped is finished by definition — the same rule the
        // stage migration applied to every released work when the dial arrived.
        if release_status == release::RELEASED {
            conn.execute(
                "UPDATE work SET stage = 100 WHERE id = ?1",
                params![created.id],
            )?;
        }

        status::refresh(conn, config, &created.id)?;
        sources.insert(source_id);
        moved += 1;
    }

    // The song lost a fact — the release that made it "released" or
    // "scheduled" now speaks for another work — so its status is derived
    // again. A status a person pinned is stepped over, as always.
    for source_id in sources {
        status::refresh(conn, config, &source_id)?;
    }

    Ok(moved)
}

/// Take the moved doors off the song and the instrumental in the stored
/// document. The tiers are left alone: a song's `clip` tier is a judgement
/// of the song, not a door. Returns whether anything changed.
fn strip_doors(config: &mut ProfileConfig) -> bool {
    let movable: Vec<&str> = MOVES
        .iter()
        .filter(|mv| target_ready(config, mv))
        .map(|mv| mv.door)
        .collect();
    let mut changed = false;
    for kind in &mut config.work_kinds {
        if !SOURCE_KINDS.contains(&kind.key.as_str()) {
            continue;
        }
        let before = kind.release_kinds.len();
        kind.release_kinds
            .retain(|door| !movable.contains(&door.key.as_str()));
        changed |= kind.release_kinds.len() != before;
    }
    changed
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;
    use crate::profile::{self, config::ReleaseKind};
    use crate::release::{self as releases, NewRelease, ReleasePatch};
    use crate::work::version::{self, NewVersion};
    use serde_json::json;

    /// The two doors a song had before v0.74, as an older workspace still
    /// stores them: the shipped profile no longer carries them, so a fixture
    /// has to put them back where the upgrade expects to find them.
    fn old_doors() -> Vec<ReleaseKind> {
        vec![
            ReleaseKind::new("clip", "Video clip", &["lyrics", "style"]).with_icon("film"),
            ReleaseKind::new("short", "Short", &["lyrics", "style"]).with_icon("smartphone"),
        ]
    }

    fn stored_config(conn: &Connection, profile_id: &str) -> ProfileConfig {
        profile::config_for(conn, profile_id).unwrap()
    }

    fn store_config(conn: &Connection, profile_id: &str, config: &ProfileConfig) {
        conn.execute(
            "UPDATE profile SET config = ?2 WHERE id = ?1",
            params![profile_id, serde_json::to_string(config).unwrap()],
        )
        .unwrap();
    }

    /// A workspace as one made before v0.74 looks after the seed: the song
    /// and the instrumental still list the clip and short doors.
    fn older_workspace(conn: &Connection) -> String {
        profile::seed(conn).unwrap();
        let profile_id = profile::id_for_key(conn, STUDIO).unwrap().unwrap();
        let mut config = stored_config(conn, &profile_id);
        for kind in &mut config.work_kinds {
            if SOURCE_KINDS.contains(&kind.key.as_str()) {
                let mut doors = old_doors();
                doors.append(&mut kind.release_kinds);
                kind.release_kinds = doors;
            }
        }
        store_config(conn, &profile_id, &config);
        profile_id
    }

    fn a_song(conn: &mut Connection, profile_id: &str, title: &str) -> (String, String) {
        let song = work::create(
            conn,
            profile_id,
            NewWork {
                kind: "song".into(),
                title: title.into(),
                meta: json!({ "bpm": 120, "stray": "not a field" })
                    .as_object()
                    .cloned(),
                ..NewWork::default()
            },
        )
        .unwrap();
        let lyrics = version::create(
            conn,
            &song.id,
            NewVersion {
                role: "lyrics".into(),
                body: "one line".into(),
                label: None,
                meta: None,
                make_current: true,
                parent_version_id: None,
            },
        )
        .unwrap();
        (song.id, lyrics.id)
    }

    fn a_release(conn: &Connection, work_id: &str, kind: &str) -> String {
        releases::create(
            conn,
            NewRelease {
                work_id: work_id.to_owned(),
                kind: kind.into(),
                title: Some(format!("{kind} title")),
                scheduled_at: None,
                meta: None,
                scheduled_time: None,
                time_zone: None,
            },
        )
        .unwrap()
        .id
    }

    /// A short that went out: dated, timed, pinned, with a link and the
    /// text it went out under — every column the move must not touch.
    fn a_released_short(conn: &Connection, work_id: &str) -> String {
        let id = a_release(conn, work_id, "short");
        releases::schedule(conn, &id, "2026-09-20").unwrap();
        releases::set_slot_pin(conn, &id, true).unwrap();
        releases::update(
            conn,
            &id,
            ReleasePatch {
                scheduled_time: Some(Some("18:00".into())),
                time_zone: Some(Some("UTC".into())),
                meta: json!({ "title": "The hook", "description": "Sixty seconds" })
                    .as_object()
                    .cloned(),
                ..Default::default()
            },
        )
        .unwrap();
        releases::mark_released(
            conn,
            &id,
            Some("https://example.com/s/1".into()),
            Some("2026-09-20".into()),
        )
        .unwrap();
        id
    }

    /// A release as JSON, without the three columns the move changes.
    fn everything_else(release: &releases::Release) -> Value {
        let mut value = serde_json::to_value(release).unwrap();
        let object = value.as_object_mut().unwrap();
        for key in ["work_id", "kind", "updated_at"] {
            object.remove(key);
        }
        value
    }

    fn works_of(conn: &Connection, profile_id: &str, kind: &str) -> Vec<work::Work> {
        work::list(
            conn,
            profile_id,
            &work::WorkFilter {
                kind: Some(kind.into()),
                ..Default::default()
            },
        )
        .unwrap()
    }

    fn count(conn: &Connection, table: &str) -> i64 {
        conn.query_row(&format!("SELECT count(*) FROM {table}"), [], |row| {
            row.get(0)
        })
        .unwrap()
    }

    fn doors_of(config: &ProfileConfig, kind: &str) -> Vec<String> {
        config
            .vocabulary(kind)
            .release_kinds
            .iter()
            .map(|door| door.key.clone())
            .collect()
    }

    fn journal_lines(conn: &Connection, profile_id: &str) -> Vec<journal::Entry> {
        journal::list(conn, profile_id)
            .unwrap()
            .into_iter()
            .filter(|entry| entry.action == "upgrade.doorsMoved")
            .collect()
    }

    fn stored_row(conn: &Connection, profile_id: &str) -> (String, String) {
        conn.query_row(
            "SELECT config, updated_at FROM profile WHERE id = ?1",
            params![profile_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap()
    }

    #[test]
    fn a_songs_clip_and_short_releases_move_onto_works_of_their_own() {
        let mut conn = db::open_in_memory().unwrap();
        let profile_id = older_workspace(&conn);
        let (song, lyrics) = a_song(&mut conn, &profile_id, "Harbour lights");
        let short = a_released_short(&conn, &song);
        let clip = a_release(&conn, &song, "clip");
        let audio = a_release(&conn, &song, "audio");
        let config = stored_config(&conn, &profile_id);
        status::refresh(&conn, &config, &song).unwrap();
        assert_eq!(
            work::get(&conn, &song).unwrap().unwrap().status,
            "released",
            "the short that went out spoke for the song"
        );
        let short_before = releases::get(&conn, &short).unwrap().unwrap();
        let clip_before = releases::get(&conn, &clip).unwrap().unwrap();

        assert_eq!(upgrade(&mut conn).unwrap(), 2);

        // Two new works, one per release, each made from the song at the
        // version the song is on.
        let shorts = works_of(&conn, &profile_id, "short");
        let videos = works_of(&conn, &profile_id, "video");
        assert_eq!((shorts.len(), videos.len()), (1, 1));
        for made in [&shorts[0], &videos[0]] {
            assert_eq!(made.title, "Harbour lights");
            assert_eq!(
                made.meta,
                json!({ "bpm": 120 }).as_object().cloned().unwrap(),
                "only the profile's own fields come along"
            );
            let sources = link::sources(&conn, &made.id).unwrap();
            assert_eq!(sources.len(), 1);
            assert_eq!(sources[0].source_id, song);
            assert_eq!(sources[0].role, link::DONOR);
            assert_eq!(
                sources[0].source_version_id.as_deref(),
                Some(lyrics.as_str())
            );
        }

        // The releases changed hands and door, and nothing else.
        let short_after = releases::get(&conn, &short).unwrap().unwrap();
        assert_eq!(short_after.work_id, shorts[0].id);
        assert_eq!(short_after.kind, "short");
        assert_eq!(
            everything_else(&short_after),
            everything_else(&short_before)
        );
        let clip_after = releases::get(&conn, &clip).unwrap().unwrap();
        assert_eq!(clip_after.work_id, videos[0].id);
        assert_eq!(clip_after.kind, "youtube");
        assert_eq!(everything_else(&clip_after), everything_else(&clip_before));
        let audio_after = releases::get(&conn, &audio).unwrap().unwrap();
        assert_eq!(
            audio_after.work_id, song,
            "the audio release is the song's own door"
        );
        assert_eq!(audio_after.kind, "audio");

        // The facts moved with the releases, and every status followed them.
        assert_eq!(shorts[0].status, "released");
        assert_eq!(
            shorts[0].stage,
            Some(100),
            "a short that went out is finished"
        );
        assert_eq!(videos[0].status, "draft");
        assert_eq!(videos[0].stage, None);
        assert_eq!(
            work::get(&conn, &song).unwrap().unwrap().status,
            "draft",
            "nothing of the song's own has gone out"
        );

        // The doors are off the song; the tier of the same name is not.
        let config = stored_config(&conn, &profile_id);
        assert_eq!(doors_of(&config, "song"), vec!["audio"]);
        assert_eq!(doors_of(&config, "instrumental"), vec!["audio"]);
        assert!(
            config
                .vocabulary("song")
                .tiers
                .iter()
                .any(|tier| tier.key == "clip"),
            "a song's clip tier is a judgement, not a door"
        );

        let lines = journal_lines(&conn, &profile_id);
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].params["count"], 2);
    }

    #[test]
    fn running_the_upgrade_again_changes_nothing() {
        let mut conn = db::open_in_memory().unwrap();
        let profile_id = older_workspace(&conn);
        let (song, _) = a_song(&mut conn, &profile_id, "Twice");
        a_released_short(&conn, &song);
        a_release(&conn, &song, "clip");
        upgrade(&mut conn).unwrap();
        let works = count(&conn, "work");
        let releases = count(&conn, "release");
        let links = count(&conn, "work_link");
        let row = stored_row(&conn, &profile_id);

        assert_eq!(upgrade(&mut conn).unwrap(), 0);

        assert_eq!(count(&conn, "work"), works);
        assert_eq!(count(&conn, "release"), releases);
        assert_eq!(count(&conn, "work_link"), links);
        assert_eq!(journal_lines(&conn, &profile_id).len(), 1, "no second line");
        assert_eq!(
            stored_row(&conn, &profile_id),
            row,
            "the document is not rewritten for nothing"
        );
    }

    #[test]
    fn a_fresh_workspace_has_nothing_to_move_and_is_left_byte_for_byte() {
        let mut conn = db::open_in_memory().unwrap();
        profile::seed(&conn).unwrap();
        let profile_id = profile::id_for_key(&conn, STUDIO).unwrap().unwrap();
        let before = stored_row(&conn, &profile_id);

        assert_eq!(upgrade(&mut conn).unwrap(), 0);

        assert_eq!(stored_row(&conn, &profile_id), before);
        assert!(journal_lines(&conn, &profile_id).is_empty());
    }

    /// Two shorts of one song are two pieces: each gets a work of its own.
    #[test]
    fn every_release_becomes_a_work_of_its_own() {
        let mut conn = db::open_in_memory().unwrap();
        let profile_id = older_workspace(&conn);
        let (song, _) = a_song(&mut conn, &profile_id, "Two shorts");
        let first = a_release(&conn, &song, "short");
        let second = a_release(&conn, &song, "short");

        assert_eq!(upgrade(&mut conn).unwrap(), 2);

        assert_eq!(works_of(&conn, &profile_id, "short").len(), 2);
        let owners: BTreeSet<String> = [first, second]
            .iter()
            .map(|id| releases::get(&conn, id).unwrap().unwrap().work_id)
            .collect();
        assert_eq!(owners.len(), 2, "one work per release, not one per song");
        assert_eq!(link::derived(&conn, &song).unwrap().len(), 2);
    }

    /// A release with nowhere to go is left where it is, door and all — and
    /// the other door still moves.
    #[test]
    fn a_door_whose_target_kind_the_owner_removed_is_left_alone() {
        let mut conn = db::open_in_memory().unwrap();
        let profile_id = older_workspace(&conn);
        let mut config = stored_config(&conn, &profile_id);
        config.work_kinds.retain(|kind| kind.key != "video");
        store_config(&conn, &profile_id, &config);
        let (song, _) = a_song(&mut conn, &profile_id, "No video kind");
        let clip = a_release(&conn, &song, "clip");
        let short = a_release(&conn, &song, "short");

        assert_eq!(upgrade(&mut conn).unwrap(), 1);

        let clip_after = releases::get(&conn, &clip).unwrap().unwrap();
        assert_eq!(
            (clip_after.work_id.as_str(), clip_after.kind.as_str()),
            (song.as_str(), "clip")
        );
        assert_eq!(releases::get(&conn, &short).unwrap().unwrap().kind, "short");
        assert!(works_of(&conn, &profile_id, "video").is_empty());
        let config = stored_config(&conn, &profile_id);
        assert_eq!(
            doors_of(&config, "song"),
            vec!["clip", "audio"],
            "the door with nowhere to go stays, the other is off"
        );
        assert_eq!(journal_lines(&conn, &profile_id)[0].params["count"], 1);
    }

    /// The rule is about the doors kilna shipped: a profile the owner
    /// invented keeps whatever doors it gives a song.
    #[test]
    fn a_profile_of_the_owners_own_is_not_touched() {
        let mut conn = db::open_in_memory().unwrap();
        let profile_id = older_workspace(&conn);
        conn.execute(
            "INSERT INTO profile (id, key, name, config, is_active, is_builtin, created_at, updated_at)
             SELECT 'mine', 'mine', 'Mine', config, 0, 0, created_at, updated_at
               FROM profile WHERE id = ?1",
            params![profile_id],
        )
        .unwrap();
        let (song, _) = a_song(&mut conn, "mine", "Theirs");
        let clip = a_release(&conn, &song, "clip");

        assert_eq!(upgrade(&mut conn).unwrap(), 0);

        let after = releases::get(&conn, &clip).unwrap().unwrap();
        assert_eq!(
            (after.work_id.as_str(), after.kind.as_str()),
            (song.as_str(), "clip")
        );
        assert_eq!(
            doors_of(&stored_config(&conn, "mine"), "song"),
            vec!["clip", "short", "audio"]
        );
        assert!(journal_lines(&conn, "mine").is_empty());
    }

    /// The whole point is that it happens when the workspace opens, without
    /// anybody asking for it.
    #[test]
    fn the_move_happens_when_the_workspace_opens() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("workspace.db");
        let (song, clip) = {
            let mut conn = db::open(&path).unwrap();
            let profile_id = older_workspace(&conn);
            let (song, _) = a_song(&mut conn, &profile_id, "Opened");
            let clip = a_release(&conn, &song, "clip");
            (song, clip)
        };

        let state = crate::state::AppState::open(&path).unwrap();

        let conn = state.conn();
        let after = releases::get(&conn, &clip).unwrap().unwrap();
        assert_ne!(after.work_id, song);
        assert_eq!(after.kind, "youtube");
        assert_eq!(
            work::get(&conn, &after.work_id).unwrap().unwrap().kind,
            "video"
        );
        let profile_id = profile::id_for_key(&conn, STUDIO).unwrap().unwrap();
        assert_eq!(
            doors_of(&stored_config(&conn, &profile_id), "song"),
            vec!["audio"]
        );
    }
}
