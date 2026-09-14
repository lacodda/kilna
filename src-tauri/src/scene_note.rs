//! What a scene is about: the people in it, the places it happens.
//!
//! A scene names notes rather than describing them again. "Every scene with
//! her in it" is a question a board has to answer, and a name written three
//! ways across fifty descriptions is three people to anything that reads
//! them. The thing pointed at is a note of a kind the profile names —
//! `character`, `location` — because a character *is* a note with an editing
//! screen of its own coming later (v0.73), not a second row meaning the same
//! person.

use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use crate::minted::Minted;

/// A note a scene points at, as the board shows it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceneNote {
    pub id: String,
    pub scene_id: String,
    pub note_id: String,
    /// The note's own kind — `character`, `location` — so the board can group
    /// what a scene is about without reading every note.
    pub note_kind: String,
    pub note_title: Option<String>,
    pub created_at: String,
}

const SELECT: &str = "SELECT sn.id, sn.scene_id, sn.note_id, n.kind, n.title, sn.created_at \
     FROM scene_note sn JOIN note n ON n.id = sn.note_id";

/// Point a scene at a note.
pub fn attach(conn: &Connection, scene_id: &str, note_id: &str) -> Result<SceneNote> {
    attach_minted(conn, scene_id, note_id, Minted::fresh())
}

/// Point a scene at a note with the id and moment already decided — the seam
/// a replay comes back through, see ADR 0014.
pub fn attach_minted(
    conn: &Connection,
    scene_id: &str,
    note_id: &str,
    minted: Minted,
) -> Result<SceneNote> {
    let scene =
        crate::scene::get(conn, scene_id)?.ok_or_else(|| Error::not_found("scene", scene_id))?;
    let note = crate::note::get(conn, note_id)?.ok_or_else(|| Error::not_found("note", note_id))?;

    if note.profile_id != scene.profile_id {
        return Err(Error::Other(
            "a scene is about a note of the same profile".into(),
        ));
    }
    check_kind(conn, &scene.profile_id, &note.kind)?;

    conn.execute(
        "INSERT INTO scene_note (id, profile_id, scene_id, note_id, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5)
         ON CONFLICT (scene_id, note_id) DO NOTHING",
        params![
            minted.id(),
            scene.profile_id,
            scene_id,
            note_id,
            minted.at()
        ],
    )?;

    // Already there: the row that is there is the answer, not an error. A
    // person naming the same character twice means the character is in the
    // scene, which it already was.
    get_for(conn, scene_id, note_id)?.ok_or_else(|| Error::not_found("scene_note", minted.id()))
}

/// Stop a scene pointing at a note.
pub fn detach(conn: &Connection, scene_id: &str, note_id: &str) -> Result<()> {
    conn.execute(
        "DELETE FROM scene_note WHERE scene_id = ?1 AND note_id = ?2",
        params![scene_id, note_id],
    )?;
    Ok(())
}

/// What one scene is about, oldest first.
pub fn for_scene(conn: &Connection, scene_id: &str) -> Result<Vec<SceneNote>> {
    let mut statement = conn.prepare(&format!(
        "{SELECT} WHERE sn.scene_id = ?1 ORDER BY sn.created_at, sn.rowid"
    ))?;
    let rows = statement
        .query_map(params![scene_id], read)?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows)
}

/// What every scene of a board is about, in one read: the board draws fifty
/// rows, and fifty queries to fill them is the shape that made the
/// predecessor's screens slow.
pub fn for_work(conn: &Connection, work_id: &str) -> Result<Vec<SceneNote>> {
    let mut statement = conn.prepare(&format!(
        "{SELECT} JOIN scene s ON s.id = sn.scene_id
         WHERE s.work_id = ?1 ORDER BY s.position, sn.created_at, sn.rowid"
    ))?;
    let rows = statement
        .query_map(params![work_id], read)?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows)
}

/// The scenes a note is in — "every scene with her in it", from the other end.
pub fn scenes_with(conn: &Connection, note_id: &str) -> Result<Vec<String>> {
    let mut statement = conn.prepare(
        "SELECT sn.scene_id FROM scene_note sn
         JOIN scene s ON s.id = sn.scene_id
         WHERE sn.note_id = ?1 ORDER BY s.position",
    )?;
    let rows = statement
        .query_map(params![note_id], |row| row.get::<_, String>(0))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows)
}

fn get_for(conn: &Connection, scene_id: &str, note_id: &str) -> Result<Option<SceneNote>> {
    let mut statement = conn.prepare(&format!(
        "{SELECT} WHERE sn.scene_id = ?1 AND sn.note_id = ?2"
    ))?;
    let row = statement
        .query_row(params![scene_id, note_id], read)
        .optional()?;
    Ok(row)
}

/// A note a scene may point at is one of the kinds the profile names.
///
/// Checked here rather than in the schema: the vocabulary is the profile's
/// and changes while the workspace lives, and a database that refused a note
/// whose kind was renamed yesterday would lose the link rather than the word.
/// A profile naming no kinds of note has not decided yet, and anything goes —
/// the same leniency a kind with no `shot_types` gets.
fn check_kind(conn: &Connection, profile_id: &str, kind: &str) -> Result<()> {
    let config = crate::profile::config_for(conn, profile_id)?;
    if config.note_kinds.is_empty() || config.note_kinds.iter().any(|one| one.key == kind) {
        return Ok(());
    }
    let named: Vec<String> = config
        .note_kinds
        .iter()
        .map(|one| format!("`{}`", one.key))
        .collect();
    Err(Error::Other(format!(
        "a scene is about a note of a kind the profile names: {}, not `{kind}`",
        named.join(", ")
    )))
}

fn read(row: &rusqlite::Row<'_>) -> rusqlite::Result<SceneNote> {
    Ok(SceneNote {
        id: row.get(0)?,
        scene_id: row.get(1)?,
        note_id: row.get(2)?,
        note_kind: row.get(3)?,
        note_title: row.get(4)?,
        created_at: row.get(5)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::profile::config::Kind;
    use crate::work::{self, NewWork};
    use crate::{db, note, profile, scene};

    fn workspace() -> (Connection, String) {
        let conn = db::open_in_memory().unwrap();
        profile::seed(&conn).unwrap();
        let profile_id = profile::active(&conn).unwrap().unwrap().id;
        (conn, profile_id)
    }

    fn with_note_kinds(conn: &Connection, profile_id: &str) {
        let mut config = profile::config_for(conn, profile_id).unwrap();
        config.note_kinds = vec![
            Kind::new("character", "Character"),
            Kind::new("location", "Location"),
        ];
        profile::update_config(conn, profile_id, &config).unwrap();
    }

    fn a_scene(conn: &Connection, profile_id: &str) -> String {
        let work_id = work::create(
            conn,
            profile_id,
            NewWork {
                kind: "video".into(),
                title: "Harbour lights".into(),
                ..NewWork::default()
            },
        )
        .unwrap()
        .id;
        scene::create(
            conn,
            profile_id,
            scene::NewScene {
                work_id,
                ..scene::NewScene::default()
            },
        )
        .unwrap()
        .id
    }

    fn a_note(conn: &Connection, profile_id: &str, kind: &str) -> String {
        note::create(
            conn,
            profile_id,
            note::NewNote {
                body: String::new(),
                kind: Some(kind.into()),
                title: Some("Her".into()),
                work_id: None,
                tags: Vec::new(),
            },
        )
        .unwrap()
        .id
    }

    /// A scene points at a character, and the board can ask the question back:
    /// which scenes is she in.
    #[test]
    fn a_scene_names_who_is_in_it_and_the_note_names_its_scenes() {
        let (conn, profile_id) = workspace();
        with_note_kinds(&conn, &profile_id);
        let scene_id = a_scene(&conn, &profile_id);
        let note_id = a_note(&conn, &profile_id, "character");

        let link = attach(&conn, &scene_id, &note_id).unwrap();
        assert_eq!(link.note_kind, "character");
        assert_eq!(link.note_title.as_deref(), Some("Her"));

        assert_eq!(for_scene(&conn, &scene_id).unwrap().len(), 1);
        assert_eq!(
            scenes_with(&conn, &note_id).unwrap(),
            vec![scene_id.clone()]
        );

        detach(&conn, &scene_id, &note_id).unwrap();
        assert!(for_scene(&conn, &scene_id).unwrap().is_empty());
        assert!(scenes_with(&conn, &note_id).unwrap().is_empty());
    }

    /// Naming the same character twice means she is in the scene, which she
    /// already was: the second naming is the first, not an error and not a
    /// duplicate row.
    #[test]
    fn naming_the_same_note_twice_is_the_same_link() {
        let (conn, profile_id) = workspace();
        with_note_kinds(&conn, &profile_id);
        let scene_id = a_scene(&conn, &profile_id);
        let note_id = a_note(&conn, &profile_id, "character");

        let first = attach(&conn, &scene_id, &note_id).unwrap();
        let again = attach(&conn, &scene_id, &note_id).unwrap();
        assert_eq!(first.id, again.id);
        assert_eq!(for_scene(&conn, &scene_id).unwrap().len(), 1);
    }

    /// A note of a kind the profile does not name is refused, and the refusal
    /// says which kinds it does name — the way a kind of shot is refused.
    #[test]
    fn a_note_of_another_kind_is_refused_by_name() {
        let (conn, profile_id) = workspace();
        with_note_kinds(&conn, &profile_id);
        let scene_id = a_scene(&conn, &profile_id);
        let plain = a_note(&conn, &profile_id, "note");

        let refused = attach(&conn, &scene_id, &plain).unwrap_err().to_string();
        assert!(refused.contains("character"), "names the kinds: {refused}");
        assert!(
            refused.contains("`note`"),
            "and what was offered: {refused}"
        );
    }

    /// A profile that names no kinds of note has not decided yet, and anything
    /// goes — the leniency a kind with no kinds of shot gets.
    #[test]
    fn a_profile_naming_no_kinds_accepts_any_note() {
        let (conn, profile_id) = workspace();
        let scene_id = a_scene(&conn, &profile_id);
        let plain = a_note(&conn, &profile_id, "note");
        assert!(attach(&conn, &scene_id, &plain).is_ok());
    }

    /// The board reads what every scene is about in one go.
    #[test]
    fn a_board_reads_what_its_scenes_are_about_at_once() {
        let (conn, profile_id) = workspace();
        with_note_kinds(&conn, &profile_id);
        let work_id = work::create(
            &conn,
            &profile_id,
            NewWork {
                kind: "video".into(),
                title: "Harbour lights".into(),
                ..NewWork::default()
            },
        )
        .unwrap()
        .id;
        let her = a_note(&conn, &profile_id, "character");
        let there = a_note(&conn, &profile_id, "location");

        for _ in 0..2 {
            let scene_id = scene::create(
                &conn,
                &profile_id,
                scene::NewScene {
                    work_id: work_id.clone(),
                    ..scene::NewScene::default()
                },
            )
            .unwrap()
            .id;
            attach(&conn, &scene_id, &her).unwrap();
            attach(&conn, &scene_id, &there).unwrap();
        }

        let all = for_work(&conn, &work_id).unwrap();
        assert_eq!(all.len(), 4, "two scenes, two notes each");
        assert_eq!(scenes_with(&conn, &her).unwrap().len(), 2);
    }

    /// Deleting the scene takes what it was about with it; the notes stay.
    #[test]
    fn deleting_a_scene_leaves_the_notes_alone() {
        let (mut conn, profile_id) = workspace();
        with_note_kinds(&conn, &profile_id);
        let scene_id = a_scene(&conn, &profile_id);
        let note_id = a_note(&conn, &profile_id, "character");
        attach(&conn, &scene_id, &note_id).unwrap();

        scene::delete(&conn, &scene_id).unwrap();
        assert!(scenes_with(&conn, &note_id).unwrap().is_empty());
        assert!(
            note::get(&conn, &note_id).unwrap().is_some(),
            "the character outlives the scene she was in"
        );
        let _ = &mut conn;
    }
}
