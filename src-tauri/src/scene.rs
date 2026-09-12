//! A scene: one row of a video's storyboard.
//!
//! A scene belongs to the work — not to a release and not to a version
//! (decision of 2026-09-11): a second attempt at a video is a second video
//! made from the same donor, and the scenes go with the work they are the
//! storyboard of. Each scene is a row with fields — a number, the part of the
//! text it plays against, its seconds, the kind of shot, a description — and
//! a set of prompt blocks keyed by the kind's `scene_blocks`, each edited on
//! its own and copied on its own. The predecessor kept a scene as a markdown
//! file edited whole; here the scaffold is the schema's and a person edits
//! inside a block. See ADR 0020.

use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use serde_json::Map;

use crate::error::{Error, Result};
use crate::minted::Minted;
use crate::profile::config::{ProfileConfig, WorkKind};
use crate::time::now;

/// The prompt blocks of a scene, by the kind's block key.
pub type Blocks = Map<String, serde_json::Value>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Scene {
    pub id: String,
    pub profile_id: String,
    pub work_id: String,
    /// The scene's number on the board, from 1.
    pub position: i64,
    /// The part of the text it plays against: intro, verse 1, chorus.
    pub section: Option<String>,
    /// Seconds from the start of the video; unset until the board is timed.
    pub starts_at: Option<f64>,
    pub ends_at: Option<f64>,
    /// A key of the kind's `shot_types`; unset when not yet decided.
    pub shot_type: Option<String>,
    pub description: String,
    /// Prompt blocks by the kind's `scene_blocks` key.
    pub blocks: Blocks,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NewScene {
    pub work_id: String,
    /// After the last scene when omitted.
    #[serde(default)]
    pub position: Option<i64>,
    #[serde(default)]
    pub section: Option<String>,
    #[serde(default)]
    pub starts_at: Option<f64>,
    #[serde(default)]
    pub ends_at: Option<f64>,
    #[serde(default)]
    pub shot_type: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub blocks: Option<Blocks>,
}

/// What an edit may change. `blocks` replaces the whole set: the screen
/// edits one block and sends them all, so the log's `before` holds the set
/// as it was and an undo puts the set back.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ScenePatch {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub position: Option<i64>,
    #[serde(
        default,
        deserialize_with = "crate::reversal::nullable",
        skip_serializing_if = "Option::is_none"
    )]
    pub section: Option<Option<String>>,
    #[serde(
        default,
        deserialize_with = "crate::reversal::nullable",
        skip_serializing_if = "Option::is_none"
    )]
    pub starts_at: Option<Option<f64>>,
    #[serde(
        default,
        deserialize_with = "crate::reversal::nullable",
        skip_serializing_if = "Option::is_none"
    )]
    pub ends_at: Option<Option<f64>>,
    #[serde(
        default,
        deserialize_with = "crate::reversal::nullable",
        skip_serializing_if = "Option::is_none"
    )]
    pub shot_type: Option<Option<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub blocks: Option<Blocks>,
}

const SELECT_SCENE: &str = "SELECT id, profile_id, work_id, position, section, starts_at, ends_at, \
     shot_type, description, blocks, created_at, updated_at FROM scene";

pub fn create(conn: &Connection, profile_id: &str, new: NewScene) -> Result<Scene> {
    create_minted(conn, profile_id, new, Minted::fresh())
}

/// Make a scene with the id and moment already decided — the seam a replay
/// comes back through, see ADR 0014.
///
/// The work must be in the profile, and the kind of shot and the block keys
/// must be words of the work's kind: a scene typed as `closeup` when the
/// vocabulary says `close` would never be found by the filter, and a block
/// under a key no template names would never be read.
pub fn create_minted(
    conn: &Connection,
    profile_id: &str,
    new: NewScene,
    minted: Minted,
) -> Result<Scene> {
    let work = crate::work::get(conn, &new.work_id)?
        .ok_or_else(|| Error::not_found("work", &new.work_id))?;
    if work.profile_id != profile_id {
        return Err(Error::Other(
            "a scene belongs to a work of the same profile".into(),
        ));
    }
    let config = crate::profile::config_for(conn, profile_id)?;
    let kind = config.vocabulary(&work.kind);

    let shot_type = clean(new.shot_type);
    check_shot_type(kind, shot_type.as_deref())?;
    let blocks = new.blocks.unwrap_or_default();
    check_blocks(kind, &blocks)?;
    let position = match new.position {
        Some(position) if position >= 1 => position,
        Some(_) => return Err(Error::Other("a scene is numbered from 1".into())),
        None => next_position(conn, &work.id)?,
    };
    check_span(new.starts_at, new.ends_at)?;

    let id = minted.id().to_owned();
    conn.execute(
        "INSERT INTO scene (id, profile_id, work_id, position, section, starts_at, ends_at, \
         shot_type, description, blocks, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?11)",
        params![
            id,
            profile_id,
            work.id,
            position,
            clean(new.section),
            new.starts_at,
            new.ends_at,
            shot_type,
            new.description.unwrap_or_default(),
            serde_json::to_string(&blocks)?,
            minted.at(),
        ],
    )?;

    get(conn, &id)?.ok_or_else(|| Error::Other("the scene vanished after insert".into()))
}

pub fn get(conn: &Connection, id: &str) -> Result<Option<Scene>> {
    let raw = conn
        .query_row(
            &format!("{SELECT_SCENE} WHERE id = ?1"),
            params![id],
            read_row,
        )
        .optional()?;
    raw.map(RawScene::into_scene).transpose()
}

/// The storyboard of a work, in order: by number, then as inserted.
pub fn for_work(conn: &Connection, work_id: &str) -> Result<Vec<Scene>> {
    let mut statement = conn.prepare(&format!(
        "{SELECT_SCENE} WHERE work_id = ?1 ORDER BY position, rowid"
    ))?;
    let raw = statement
        .query_map(params![work_id], read_row)?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    raw.into_iter().map(RawScene::into_scene).collect()
}

/// How many scenes a work has.
pub fn count(conn: &Connection, work_id: &str) -> Result<i64> {
    Ok(conn.query_row(
        "SELECT count(*) FROM scene WHERE work_id = ?1",
        params![work_id],
        |row| row.get(0),
    )?)
}

pub fn update(conn: &Connection, id: &str, patch: ScenePatch) -> Result<Scene> {
    update_at(conn, id, patch, &now())
}

/// Edit a scene with the change's moment already decided — the seam a
/// replay comes back through, see ADR 0014.
pub fn update_at(conn: &Connection, id: &str, patch: ScenePatch, at: &str) -> Result<Scene> {
    let before = get(conn, id)?.ok_or_else(|| unknown_scene(id))?;
    let config = crate::profile::config_for(conn, &before.profile_id)?;
    let work = crate::work::get(conn, &before.work_id)?
        .ok_or_else(|| Error::not_found("work", &before.work_id))?;
    let kind = config.vocabulary(&work.kind);

    let mut assignments: Vec<String> = Vec::new();
    let mut values: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

    fn set(
        assignments: &mut Vec<String>,
        values: &mut Vec<Box<dyn rusqlite::ToSql>>,
        column: &str,
        value: Box<dyn rusqlite::ToSql>,
    ) {
        values.push(value);
        assignments.push(format!("{column} = ?{}", values.len()));
    }

    if let Some(position) = patch.position {
        if position < 1 {
            return Err(Error::Other("a scene is numbered from 1".into()));
        }
        set(
            &mut assignments,
            &mut values,
            "position",
            Box::new(position),
        );
    }
    if let Some(section) = patch.section {
        set(
            &mut assignments,
            &mut values,
            "section",
            Box::new(clean(section)),
        );
    }
    let starts_at = patch.starts_at.unwrap_or(before.starts_at);
    let ends_at = patch.ends_at.unwrap_or(before.ends_at);
    check_span(starts_at, ends_at)?;
    if let Some(starts_at) = patch.starts_at {
        set(
            &mut assignments,
            &mut values,
            "starts_at",
            Box::new(starts_at),
        );
    }
    if let Some(ends_at) = patch.ends_at {
        set(&mut assignments, &mut values, "ends_at", Box::new(ends_at));
    }
    if let Some(shot_type) = patch.shot_type {
        let shot_type = clean(shot_type);
        check_shot_type(kind, shot_type.as_deref())?;
        set(
            &mut assignments,
            &mut values,
            "shot_type",
            Box::new(shot_type),
        );
    }
    if let Some(description) = patch.description {
        set(
            &mut assignments,
            &mut values,
            "description",
            Box::new(description),
        );
    }
    if let Some(blocks) = patch.blocks {
        check_blocks(kind, &blocks)?;
        set(
            &mut assignments,
            &mut values,
            "blocks",
            Box::new(serde_json::to_string(&blocks)?),
        );
    }

    if assignments.is_empty() {
        return Ok(before);
    }

    set(
        &mut assignments,
        &mut values,
        "updated_at",
        Box::new(at.to_owned()),
    );
    values.push(Box::new(id.to_owned()));

    let sql = format!(
        "UPDATE scene SET {} WHERE id = ?{}",
        assignments.join(", "),
        values.len()
    );
    let params = rusqlite::params_from_iter(values.iter().map(AsRef::as_ref));
    if conn.execute(&sql, params)? == 0 {
        return Err(unknown_scene(id));
    }

    get(conn, id)?.ok_or_else(|| unknown_scene(id))
}

pub fn delete(conn: &Connection, id: &str) -> Result<()> {
    if conn.execute("DELETE FROM scene WHERE id = ?1", params![id])? == 0 {
        return Err(unknown_scene(id));
    }
    Ok(())
}

/// The number after the last scene of a work: 1 on an empty board.
fn next_position(conn: &Connection, work_id: &str) -> Result<i64> {
    let last: Option<i64> = conn.query_row(
        "SELECT max(position) FROM scene WHERE work_id = ?1",
        params![work_id],
        |row| row.get(0),
    )?;
    Ok(last.unwrap_or(0) + 1)
}

/// A kind of shot the kind names, or none.
fn check_shot_type(kind: &WorkKind, shot_type: Option<&str>) -> Result<()> {
    let Some(shot_type) = shot_type else {
        return Ok(());
    };
    if kind.shot_types.iter().any(|shot| shot.key == shot_type) {
        return Ok(());
    }
    Err(Error::Other(format!(
        "`{shot_type}` is not a kind of shot for a {}; the profile names {}",
        kind.label.to_lowercase(),
        name_keys(kind.shot_types.iter().map(|shot| shot.key.as_str()))
    )))
}

/// Blocks under keys the kind names, holding text.
fn check_blocks(kind: &WorkKind, blocks: &Blocks) -> Result<()> {
    for (key, value) in blocks {
        if !kind.scene_blocks.iter().any(|block| block.key == *key) {
            return Err(Error::Other(format!(
                "`{key}` is not a prompt block for a {}; the profile names {}",
                kind.label.to_lowercase(),
                name_keys(kind.scene_blocks.iter().map(|block| block.key.as_str()))
            )));
        }
        if !value.is_string() {
            return Err(Error::Other(format!("the block `{key}` must hold text")));
        }
    }
    Ok(())
}

/// A span that ends after it starts, when both ends are given.
fn check_span(starts_at: Option<f64>, ends_at: Option<f64>) -> Result<()> {
    for at in [starts_at, ends_at].into_iter().flatten() {
        if !(at.is_finite() && at >= 0.0) {
            return Err(Error::Other("a scene's seconds count from zero".into()));
        }
    }
    if let (Some(starts), Some(ends)) = (starts_at, ends_at) {
        if ends < starts {
            return Err(Error::Other("a scene cannot end before it starts".into()));
        }
    }
    Ok(())
}

fn name_keys<'a>(keys: impl Iterator<Item = &'a str>) -> String {
    let named: Vec<String> = keys.map(|key| format!("`{key}`")).collect();
    if named.is_empty() {
        "none".to_owned()
    } else {
        named.join(", ")
    }
}

/// Trimmed, and none when empty: a blank section is no section.
fn clean(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
}

fn unknown_scene(id: &str) -> Error {
    Error::not_found("scene", id)
}

struct RawScene {
    id: String,
    profile_id: String,
    work_id: String,
    position: i64,
    section: Option<String>,
    starts_at: Option<f64>,
    ends_at: Option<f64>,
    shot_type: Option<String>,
    description: String,
    blocks: String,
    created_at: String,
    updated_at: String,
}

fn read_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<RawScene> {
    Ok(RawScene {
        id: row.get(0)?,
        profile_id: row.get(1)?,
        work_id: row.get(2)?,
        position: row.get(3)?,
        section: row.get(4)?,
        starts_at: row.get(5)?,
        ends_at: row.get(6)?,
        shot_type: row.get(7)?,
        description: row.get(8)?,
        blocks: row.get(9)?,
        created_at: row.get(10)?,
        updated_at: row.get(11)?,
    })
}

impl RawScene {
    fn into_scene(self) -> Result<Scene> {
        Ok(Scene {
            blocks: serde_json::from_str(&self.blocks)?,
            id: self.id,
            profile_id: self.profile_id,
            work_id: self.work_id,
            position: self.position,
            section: self.section,
            starts_at: self.starts_at,
            ends_at: self.ends_at,
            shot_type: self.shot_type,
            description: self.description,
            created_at: self.created_at,
            updated_at: self.updated_at,
        })
    }
}

/// Whether works of a kind have a storyboard at all: the kind names kinds
/// of shot or prompt blocks. A song has neither, and no Scenes tab.
pub fn kind_has_scenes(config: &ProfileConfig, kind: &str) -> bool {
    let kind = config.vocabulary(kind);
    !kind.shot_types.is_empty() || !kind.scene_blocks.is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;
    use crate::profile;
    use crate::work::{self, NewWork};
    use serde_json::json;

    fn workspace() -> (Connection, String) {
        let conn = db::open_in_memory().unwrap();
        profile::seed(&conn).unwrap();
        let profile_id = profile::active(&conn).unwrap().unwrap().id;
        (conn, profile_id)
    }

    fn video(conn: &Connection, profile_id: &str) -> String {
        work::create(
            conn,
            profile_id,
            NewWork {
                kind: "video".into(),
                title: "Harbour lights".into(),
                ..NewWork::default()
            },
        )
        .unwrap()
        .id
    }

    fn scene(work_id: &str) -> NewScene {
        NewScene {
            work_id: work_id.into(),
            ..NewScene::default()
        }
    }

    #[test]
    fn scenes_are_numbered_after_the_last_and_read_back_in_order() {
        let (conn, profile_id) = workspace();
        let video = video(&conn, &profile_id);

        let first = create(&conn, &profile_id, scene(&video)).unwrap();
        let second = create(&conn, &profile_id, scene(&video)).unwrap();
        let fifth = create(
            &conn,
            &profile_id,
            NewScene {
                position: Some(5),
                ..scene(&video)
            },
        )
        .unwrap();
        let sixth = create(&conn, &profile_id, scene(&video)).unwrap();

        assert_eq!(first.position, 1);
        assert_eq!(second.position, 2);
        assert_eq!(fifth.position, 5);
        assert_eq!(sixth.position, 6, "after the last, not after the count");
        let board = for_work(&conn, &video).unwrap();
        assert_eq!(
            board.iter().map(|s| s.position).collect::<Vec<_>>(),
            [1, 2, 5, 6]
        );
        assert_eq!(count(&conn, &video).unwrap(), 4);
    }

    #[test]
    fn the_kind_of_shot_and_the_block_keys_are_the_kinds_words() {
        let (conn, profile_id) = workspace();
        let video = video(&conn, &profile_id);

        let made = create(
            &conn,
            &profile_id,
            NewScene {
                shot_type: Some("close".into()),
                blocks: Some(
                    json!({ "still": "a lighthouse at dusk", "negative": "text, watermark" })
                        .as_object()
                        .unwrap()
                        .clone(),
                ),
                section: Some("  chorus ".into()),
                ..scene(&video)
            },
        )
        .unwrap();
        assert_eq!(made.shot_type.as_deref(), Some("close"));
        assert_eq!(made.section.as_deref(), Some("chorus"), "trimmed");
        assert_eq!(made.blocks["still"], "a lighthouse at dusk");

        let err = create(
            &conn,
            &profile_id,
            NewScene {
                shot_type: Some("closeup".into()),
                ..scene(&video)
            },
        )
        .unwrap_err();
        assert!(err.to_string().contains("not a kind of shot"), "{err}");

        let err = create(
            &conn,
            &profile_id,
            NewScene {
                blocks: Some(json!({ "camera": "x" }).as_object().unwrap().clone()),
                ..scene(&video)
            },
        )
        .unwrap_err();
        assert!(err.to_string().contains("not a prompt block"), "{err}");

        let err = update(
            &conn,
            &made.id,
            ScenePatch {
                shot_type: Some(Some("nope".into())),
                ..ScenePatch::default()
            },
        )
        .unwrap_err();
        assert!(err.to_string().contains("not a kind of shot"), "{err}");
    }

    #[test]
    fn an_edit_touches_only_what_it_names_and_the_blocks_as_a_set() {
        let (conn, profile_id) = workspace();
        let video = video(&conn, &profile_id);
        let made = create(
            &conn,
            &profile_id,
            NewScene {
                description: Some("wide harbour".into()),
                blocks: Some(json!({ "still": "one" }).as_object().unwrap().clone()),
                ..scene(&video)
            },
        )
        .unwrap();

        let edited = update(
            &conn,
            &made.id,
            ScenePatch {
                blocks: Some(
                    json!({ "still": "one", "motion": "slow pan" })
                        .as_object()
                        .unwrap()
                        .clone(),
                ),
                starts_at: Some(Some(12.5)),
                ends_at: Some(Some(18.0)),
                ..ScenePatch::default()
            },
        )
        .unwrap();

        assert_eq!(edited.description, "wide harbour", "left alone");
        assert_eq!(edited.blocks["motion"], "slow pan");
        assert_eq!(edited.starts_at, Some(12.5));

        let cleared = update(
            &conn,
            &made.id,
            ScenePatch {
                section: Some(None),
                shot_type: Some(None),
                ..ScenePatch::default()
            },
        )
        .unwrap();
        assert!(cleared.section.is_none());
        assert!(cleared.shot_type.is_none());
    }

    #[test]
    fn a_scene_cannot_end_before_it_starts() {
        let (conn, profile_id) = workspace();
        let video = video(&conn, &profile_id);
        let err = create(
            &conn,
            &profile_id,
            NewScene {
                starts_at: Some(10.0),
                ends_at: Some(4.0),
                ..scene(&video)
            },
        )
        .unwrap_err();
        assert!(err.to_string().contains("end before"), "{err}");

        let made = create(
            &conn,
            &profile_id,
            NewScene {
                starts_at: Some(10.0),
                ..scene(&video)
            },
        )
        .unwrap();
        let err = update(
            &conn,
            &made.id,
            ScenePatch {
                ends_at: Some(Some(4.0)),
                ..ScenePatch::default()
            },
        )
        .unwrap_err();
        assert!(
            err.to_string().contains("end before"),
            "the stored start counts: {err}"
        );
    }

    #[test]
    fn a_song_has_no_storyboard_and_a_video_has() {
        let (conn, profile_id) = workspace();
        let config = profile::config_for(&conn, &profile_id).unwrap();
        assert!(kind_has_scenes(&config, "video"));
        assert!(kind_has_scenes(&config, "short"));
        assert!(!kind_has_scenes(&config, "song"));
        assert!(!kind_has_scenes(&config, "no-such-kind"));
    }

    #[test]
    fn deleting_the_work_takes_the_scenes_and_leaves_tombstones() {
        let (conn, profile_id) = workspace();
        let video = video(&conn, &profile_id);
        let made = create(&conn, &profile_id, scene(&video)).unwrap();

        conn.execute("DELETE FROM work WHERE id = ?1", params![video])
            .unwrap();

        assert!(get(&conn, &made.id).unwrap().is_none());
        let stones: i64 = conn
            .query_row(
                "SELECT count(*) FROM tombstone WHERE entity = 'scene' AND entity_id = ?1",
                params![made.id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(stones, 1);
    }

    #[test]
    fn an_edit_stamps_the_clock_of_the_field_it_changed() {
        let (conn, profile_id) = workspace();
        let video = video(&conn, &profile_id);
        let made = create(&conn, &profile_id, scene(&video)).unwrap();

        update(
            &conn,
            &made.id,
            ScenePatch {
                description: Some("a gull".into()),
                ..ScenePatch::default()
            },
        )
        .unwrap();

        let fields: Vec<String> = conn
            .prepare("SELECT field FROM field_clock WHERE entity = 'scene' AND entity_id = ?1 ORDER BY field")
            .unwrap()
            .query_map(params![made.id], |row| row.get(0))
            .unwrap()
            .collect::<rusqlite::Result<_>>()
            .unwrap();
        assert_eq!(fields, ["description"], "one field changed, one clock");
    }
}
