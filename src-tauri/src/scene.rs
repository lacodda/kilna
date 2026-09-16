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

/// Seconds as `m:ss`, with a fraction only when there is one — how a span
/// reads on a rendered board and in a prompt.
pub fn timecode(seconds: f64) -> String {
    let whole = seconds.floor();
    let minutes = (whole / 60.0) as i64;
    let rest = whole - (minutes * 60) as f64;
    let fraction = seconds - whole;
    if fraction > 0.0 {
        let digits = format!("{fraction:.2}");
        format!(
            "{minutes}:{:02}{}",
            rest as i64,
            digits[1..].trim_end_matches('0')
        )
    } else {
        format!("{minutes}:{:02}", rest as i64)
    }
}

/// The key a work's length is kept under, in the profile's meta fields.
///
/// A number of seconds since 2026-09-14: the field shipped as text holding
/// "3:45", which nothing could divide by. Migration 0018 retyped it and
/// converted what was already written.
pub const DURATION: &str = "duration";

/// A work's length in seconds, when its meta carries one.
///
/// Read leniently: the field is a number now, but a workspace whose owner
/// typed into it before the retype — or after retyping it back — can still
/// hold a string, and `3:45` there means the same length as `225`. A value
/// that means nothing readable is no length at all rather than a zero, so
/// the board is left untimed instead of being collapsed onto one instant.
pub fn duration_of(work: &crate::work::Work) -> Option<f64> {
    let value = work.meta.get(DURATION)?;
    if let Some(seconds) = value.as_f64() {
        return (seconds.is_finite() && seconds > 0.0).then_some(seconds);
    }
    let text = value.as_str()?.trim();
    if text.is_empty() {
        return None;
    }
    let mut seconds = 0.0_f64;
    for part in text.split(':') {
        let part: f64 = part.trim().parse().ok()?;
        if !part.is_finite() || part < 0.0 {
            return None;
        }
        seconds = seconds * 60.0 + part;
    }
    (seconds > 0.0).then_some(seconds)
}

/// The timings a work's length gives its board, one span per scene.
///
/// The length is divided evenly between the scenes in the order they stand
/// and handed back as spans — the first frame of the board, not the last:
/// "the duration divides the scenes proportionally, by hand from there"
/// (decision of 2026-09-11). Evenly rather than weighted, because nothing on
/// a scene yet says how long it wants to be; a beat that needs longer is
/// dragged out by the person, and the neighbours are theirs to settle.
///
/// Seconds are rounded to a tenth so the board reads in whole numbers rather
/// than in the tail of a division, and every span but the last is joined to
/// the next: a gap opened by rounding would show up in v0.69's gap check as a
/// hole nobody made. The last scene ends on the length itself.
pub fn timings(duration: f64, scenes: usize) -> Vec<(f64, f64)> {
    if scenes == 0 || !(duration.is_finite() && duration > 0.0) {
        return Vec::new();
    }
    let edge = |index: usize| -> f64 {
        if index == scenes {
            duration
        } else {
            ((duration * index as f64 / scenes as f64) * 10.0).round() / 10.0
        }
    };
    (0..scenes)
        .map(|index| (edge(index), edge(index + 1)))
        .collect()
}

/// Time every scene of a board from the work's length, in one change.
///
/// One operation rather than one per scene, for the same reason a collection's
/// contents are set in one: timing a board is a single gesture, and an undo of
/// it has to be single too — taking back a fifty-scene division one scene at a
/// time would leave the board in forty-nine states nobody asked for. The spans
/// the scenes held before travel in the operation's `before`, so the undo puts
/// exactly those back (ADR 0014).
///
/// Refuses rather than guesses when the work has no length: a board timed from
/// nothing would be fifty scenes all starting at zero.
pub fn time_board_at(
    conn: &mut Connection,
    work_id: &str,
    at: &str,
    logged: Option<crate::operation::Intent>,
) -> Result<Vec<Scene>> {
    let work = crate::work::get(conn, work_id)?.ok_or_else(|| Error::not_found("work", work_id))?;
    let duration = duration_of(&work).ok_or_else(|| {
        Error::Other("this work has no duration yet: give it one on the Overview tab".into())
    })?;

    let scenes = for_work(conn, work_id)?;
    if scenes.is_empty() {
        return Err(Error::Other("this board has no scenes to time".into()));
    }

    let spans = timings(duration, scenes.len());
    let tx = conn.transaction()?;

    if let Some(logged) = logged {
        crate::operation::record(&tx, logged)?;
    }

    for (scene, (starts_at, ends_at)) in scenes.iter().zip(spans) {
        tx.execute(
            "UPDATE scene SET starts_at = ?1, ends_at = ?2, updated_at = ?3 WHERE id = ?4",
            rusqlite::params![starts_at, ends_at, at, scene.id],
        )?;
    }

    tx.commit()?;
    for_work(conn, work_id)
}

/// Put back the spans a board held, one row each — the undo of a timing.
///
/// Takes the transaction rather than opening one: the undo writes its own
/// operation beside this change, and the two belong to the same commit.
pub fn restore_spans_in(
    tx: &Connection,
    spans: &[(String, Option<f64>, Option<f64>)],
    at: &str,
) -> Result<()> {
    for (id, starts_at, ends_at) in spans {
        tx.execute(
            "UPDATE scene SET starts_at = ?1, ends_at = ?2, updated_at = ?3 WHERE id = ?4",
            rusqlite::params![starts_at, ends_at, at, id],
        )?;
    }
    Ok(())
}

/// Number a board 1..N in the order given, in one change.
///
/// The board's numbers are *set* from a list of ids, not nudged by arithmetic:
/// "insert a scene between 7 and 8" and "drag 12 up to 3" are the same gesture
/// said twice, and both arrive here as the order the person wants. An
/// `insert_at` that did `position = position + 1 WHERE position >= n` would be
/// cheaper and would spread whatever mess it found — `position` carries no
/// UNIQUE on purpose (0016: a scene restored from the trash keeps its number
/// and must not be refused), so a board with two 7s and no 5 is a state that
/// happens, and it is exactly the state this stage exists to fix. Setting the
/// order cannot spread it: whatever the board held, afterwards it holds 1..N.
///
/// Naming every scene exactly once is the price, and it is also the check. A
/// list that forgets one or names an outsider is refused rather than played
/// against half a board, the way `scene_frame::reorder` refuses the same way.
///
/// Nothing but `position` moves. The frames and the clips hang on `scene_id`
/// (0021), and the file in `media/` is named by the asset's id (ADR 0002), so
/// no name anywhere holds a scene's number and there is nothing to rename
/// behind a shift — the pictures follow their scenes by not being tied to
/// their numbers in the first place.
///
/// The spans are left alone. A person who renumbers has changed what happens
/// when, and the seconds are theirs to settle or to divide again from the
/// length; carrying a span along with its scene would put scene 3 at 0:48
/// because it used to be scene 12, which is not a board anyone meant.
pub fn renumber(
    conn: &mut Connection,
    work_id: &str,
    ids: &[String],
    at: &str,
    logged: Option<crate::operation::Intent>,
) -> Result<Vec<Scene>> {
    let scenes = for_work(conn, work_id)?;
    check_order(&scenes, ids)?;

    let tx = conn.transaction()?;
    if let Some(logged) = logged {
        crate::operation::record(&tx, logged)?;
    }
    renumber_in(&tx, ids, at)?;
    tx.commit()?;

    for_work(conn, work_id)
}

/// Set the numbers, one row each — the body of a renumbering, and the body of
/// its undo.
///
/// Takes the transaction rather than opening one: an undo writes its own
/// operation beside the change, and the two belong to the same commit.
pub fn renumber_in(tx: &Connection, ids: &[String], at: &str) -> Result<()> {
    for (index, id) in ids.iter().enumerate() {
        tx.execute(
            "UPDATE scene SET position = ?1, updated_at = ?2 WHERE id = ?3",
            params![index as i64 + 1, at, id],
        )?;
    }
    Ok(())
}

/// Put back the numbers a board held, one row each — the undo of a
/// renumbering.
///
/// The numbers rather than the order, because the board a renumbering was
/// called on is often crooked — a hole, a twin, a number typed by hand — and
/// that is precisely the state it was called on to end. Putting it back as a
/// tidy 1..N would leave the person looking at a board they have never seen,
/// which is not taking anything back.
///
/// Takes the transaction rather than opening one: the undo writes its own
/// operation beside this change, and the two belong to the same commit.
pub fn restore_numbers_in(tx: &Connection, places: &[(String, i64)], at: &str) -> Result<()> {
    for (id, position) in places {
        tx.execute(
            "UPDATE scene SET position = ?1, updated_at = ?2 WHERE id = ?3",
            params![position, at, id],
        )?;
    }
    Ok(())
}

/// That the order names each scene of the board exactly once.
///
/// Refused rather than tolerated in either direction: a short list would leave
/// scenes holding numbers from the old order beside the new one, and a list
/// naming a scene twice would hand two rows the same number — both of them the
/// disorder this call is supposed to end.
fn check_order(scenes: &[Scene], ids: &[String]) -> Result<()> {
    let known: std::collections::BTreeSet<&str> =
        scenes.iter().map(|scene| scene.id.as_str()).collect();
    let named: std::collections::BTreeSet<&str> = ids.iter().map(String::as_str).collect();
    if named.len() != ids.len() || named != known {
        return Err(Error::Other(
            "renumbering a board names each of its scenes exactly once".into(),
        ));
    }
    Ok(())
}

/// The order that puts a scene at a number, with the rest closing up behind it.
///
/// The screen's arithmetic, kept here so the rule about what "position 8" means
/// is written once and tested: the scene is taken out of the order and put back
/// at the place asked for, counted in the board the person is looking at. A
/// number past the end means the end, and a number below 1 means the front,
/// because a person who drags past the edge means the edge.
pub fn order_moving(scenes: &[Scene], id: &str, to: i64) -> Vec<String> {
    let mut order: Vec<String> = scenes
        .iter()
        .map(|scene| scene.id.clone())
        .filter(|held| held != id)
        .collect();
    let at = (to - 1).clamp(0, order.len() as i64) as usize;
    order.insert(at, id.to_owned());
    order
}

/// How many parts the source text marks out, for a caller about to frame a
/// board from it. Reads through the same donor and role that framing does, so
/// the two cannot disagree about what they are counting.
pub fn parts_of_source(conn: &Connection, work_id: &str, role: &str) -> Result<usize> {
    let work = crate::work::get(conn, work_id)?.ok_or_else(|| Error::not_found("work", work_id))?;
    let donor = crate::link::sources(conn, work_id)?
        .into_iter()
        .find(|source| source.role == crate::link::DONOR)
        .ok_or_else(|| {
            Error::Other(format!(
                "“{}” is not made from anything yet: link its source on the Links tab first",
                work.title
            ))
        })?;
    let body = crate::work::version::latest(conn, &donor.source_id, role)?
        .map(|version| version.body)
        .unwrap_or_default();
    Ok(crate::work::version::sections(&body).len())
}

/// Build a board from the parts a text marks out for itself.
///
/// One scene per part, in the text's own order, each carrying the part's name
/// as its section — the frame a person then works on, not a finished board.
/// The text is the donor's: a video is made from a song (ADR 0019), and the
/// song's lyric is what has the parts. The description is left empty; what is
/// *seen* in a scene is not what is *sung* in it, and filling the description
/// with the lines would put words in the person's mouth that they would have
/// to delete before writing the shot.
///
/// Refuses rather than half-builds: a board that already has scenes is left
/// alone (the person chooses to replace it by emptying it first), and a text
/// with no markup says so instead of producing one scene holding everything.
/// `minted` carries one id per part, in order: the ids travel in the
/// operation so a workspace rebuilt from the log lands on the same rows
/// rather than inventing new ones. `parts` is how many the caller minted —
/// it must match what the text marks out, or the operation is refused
/// rather than played against a text that has changed since.
pub fn frame_from_text(
    conn: &mut Connection,
    work_id: &str,
    role: &str,
    minted: &[Minted],
    logged: Option<crate::operation::Intent>,
) -> Result<Vec<Scene>> {
    let work = crate::work::get(conn, work_id)?.ok_or_else(|| Error::not_found("work", work_id))?;

    if count(conn, work_id)? > 0 {
        return Err(Error::Other(
            "this board already has scenes: empty it first to build a new frame".into(),
        ));
    }

    let donor = crate::link::sources(conn, work_id)?
        .into_iter()
        .find(|source| source.role == crate::link::DONOR)
        .ok_or_else(|| {
            Error::Other(format!(
                "“{}” is not made from anything yet: link its source on the Links tab first",
                work.title
            ))
        })?;

    let body = crate::work::version::latest(conn, &donor.source_id, role)?
        .map(|version| version.body)
        .ok_or_else(|| {
            Error::Other(format!(
                "“{}” has no {role} to read the parts from",
                donor.source_title
            ))
        })?;

    let sections = crate::work::version::sections(&body);
    if sections.is_empty() {
        return Err(Error::Other(format!(
            "“{}” marks no parts: name them in the text — [Verse 1], [Chorus] — and try again",
            donor.source_title
        )));
    }
    if sections.len() != minted.len() {
        return Err(Error::Other(format!(
            "the text now marks {} parts, not {}: read it again rather than framing a board that does not match it",
            sections.len(),
            minted.len()
        )));
    }

    let profile_id = work.profile_id.clone();
    let tx = conn.transaction()?;
    if let Some(logged) = logged {
        crate::operation::record(&tx, logged)?;
    }
    for (index, (section, minted)) in sections.iter().zip(minted).enumerate() {
        create_minted(
            &tx,
            &profile_id,
            NewScene {
                work_id: work_id.to_owned(),
                position: Some(index as i64 + 1),
                section: Some(section.name.clone()),
                ..NewScene::default()
            },
            minted.clone(),
        )?;
    }
    tx.commit()?;

    for_work(conn, work_id)
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

    /// The length divides evenly, the spans join, and the last ends on the
    /// length itself — a board timed from 3:45 covers 225 seconds with no gap
    /// and no overhang.
    #[test]
    fn a_length_divides_the_board_evenly_and_joins_the_spans() {
        let spans = timings(225.0, 4);
        assert_eq!(spans.len(), 4);
        assert_eq!(spans[0].0, 0.0, "the board starts at zero");
        assert_eq!(spans[3].1, 225.0, "the last scene ends on the length");
        for pair in spans.windows(2) {
            assert_eq!(
                pair[0].1, pair[1].0,
                "a rounded span must join the next, or v0.69 would read the                  rounding as a gap nobody made"
            );
        }
    }

    /// A length that does not divide cleanly still covers the whole board:
    /// the rounding lands inside, never on the ends.
    ///
    /// 227.25 is the length that catches a rounded last edge — round it like
    /// the inner ones and the board ends at 227.2, four hundredths short of
    /// the work. The board must cover the work exactly, or the last frame is
    /// missing time nobody can see was dropped.
    #[test]
    fn a_length_that_does_not_divide_cleanly_still_covers_the_board() {
        for (duration, scenes) in [(100.0, 7), (227.25, 6), (95.25, 4)] {
            let spans = timings(duration, scenes);
            assert_eq!(spans.first().unwrap().0, 0.0, "{duration} starts at zero");
            assert_eq!(
                spans.last().unwrap().1,
                duration,
                "{duration} must be covered to its own last second"
            );
            for pair in spans.windows(2) {
                assert_eq!(pair[0].1, pair[1].0, "{duration} leaves no gap");
            }
        }
    }

    /// Nothing to divide, or nothing to divide between, times nothing — a
    /// board of scenes all starting at zero is worse than an untimed one.
    #[test]
    fn nothing_to_divide_times_nothing() {
        assert!(timings(225.0, 0).is_empty());
        assert!(timings(0.0, 4).is_empty());
        assert!(timings(-5.0, 4).is_empty());
        assert!(timings(f64::NAN, 4).is_empty());
    }

    /// The length is read as a number, and — for a workspace that still holds
    /// the text the field shipped as — as a time as well. Prose is no length.
    #[test]
    fn a_length_is_read_as_seconds_or_as_a_time() {
        let (conn, profile_id) = workspace();
        let work_id = video(&conn, &profile_id);
        let read = |value: serde_json::Value| {
            let mut meta = serde_json::Map::new();
            meta.insert(DURATION.into(), value);
            work::update(
                &conn,
                &work_id,
                work::WorkPatch {
                    meta: Some(meta),
                    ..Default::default()
                },
            )
            .unwrap();
            duration_of(&work::get(&conn, &work_id).unwrap().unwrap())
        };

        assert_eq!(read(json!(225)), Some(225.0));
        assert_eq!(
            read(json!("3:45")),
            Some(225.0),
            "a stored time still reads"
        );
        assert_eq!(read(json!("1:02:03")), Some(3723.0));
        assert_eq!(read(json!("about four minutes")), None);
        assert_eq!(read(json!("")), None);
        assert_eq!(read(json!(0)), None, "a length of nothing is no length");
    }

    /// Timing a board writes every scene; a work with no length refuses
    /// rather than collapsing the board onto one instant.
    #[test]
    fn timing_a_board_needs_a_length() {
        let (mut conn, profile_id) = workspace();
        let work_id = video(&conn, &profile_id);
        for _ in 0..3 {
            create(&conn, &profile_id, scene(&work_id)).unwrap();
        }

        let refused = time_board_at(&mut conn, &work_id, "2026-09-14T10:00:00.000Z", None);
        assert!(refused.is_err(), "no length, no timing");

        let mut meta = serde_json::Map::new();
        meta.insert(DURATION.into(), json!(90));
        work::update(
            &conn,
            &work_id,
            work::WorkPatch {
                meta: Some(meta),
                ..Default::default()
            },
        )
        .unwrap();

        let timed = time_board_at(&mut conn, &work_id, "2026-09-14T10:00:00.000Z", None).unwrap();
        assert_eq!(timed.len(), 3);
        assert_eq!(timed[0].starts_at, Some(0.0));
        assert_eq!(timed[2].ends_at, Some(90.0));
        assert_eq!(
            timed[0].ends_at, timed[1].starts_at,
            "the spans join on the board, not only in the arithmetic"
        );
    }

    /// The frame is one scene per part of the source text, in its order,
    /// carrying the part's name and nothing else: what is *seen* in a scene
    /// is not what is *sung* in it, and a description filled with the lines
    /// would be words the person has to delete before writing the shot.
    #[test]
    fn a_board_is_framed_from_the_parts_of_its_source() {
        let (mut conn, profile_id) = workspace();
        let song_id = work::create(
            &conn,
            &profile_id,
            NewWork {
                kind: "song".into(),
                title: "Harbour lights".into(),
                ..NewWork::default()
            },
        )
        .unwrap()
        .id;
        crate::work::version::create(
            &mut conn,
            &song_id,
            crate::work::version::NewVersion {
                role: "lyrics".into(),
                body: "[Intro]

[Verse 1]
a line

[Chorus]
the hook
"
                .into(),
                label: None,
                meta: None,
                make_current: true,
                parent_version_id: None,
            },
        )
        .unwrap();

        let video_id = video(&conn, &profile_id);

        // No source yet: framing says where to give it one.
        let orphan = crate::scene::parts_of_source(&conn, &video_id, "lyrics");
        assert!(
            orphan.is_err(),
            "a video made from nothing cannot be framed"
        );

        crate::link::create(
            &conn,
            &profile_id,
            crate::link::NewLink {
                work_id: video_id.clone(),
                source_id: song_id.clone(),
                role: None,
                source_version_id: None,
            },
        )
        .unwrap();

        let parts = crate::scene::parts_of_source(&conn, &video_id, "lyrics").unwrap();
        assert_eq!(parts, 3);
        let minted: Vec<Minted> = (0..parts).map(|_| Minted::fresh()).collect();
        let framed = frame_from_text(&mut conn, &video_id, "lyrics", &minted, None).unwrap();

        assert_eq!(
            framed
                .iter()
                .map(|scene| scene.section.as_deref().unwrap_or_default())
                .collect::<Vec<_>>(),
            ["Intro", "Verse 1", "Chorus"],
            "one scene per part, in the text's order"
        );
        assert_eq!(framed[0].position, 1);
        assert_eq!(framed[2].position, 3);
        assert!(
            framed.iter().all(|scene| scene.description.is_empty()),
            "the lines of the song are not the description of the shot"
        );

        // A board that already has scenes is left alone rather than doubled.
        let minted: Vec<Minted> = (0..parts).map(|_| Minted::fresh()).collect();
        let refused = frame_from_text(&mut conn, &video_id, "lyrics", &minted, None);
        assert!(refused.is_err(), "a board with scenes is not framed again");
        assert_eq!(count(&conn, &video_id).unwrap(), 3, "and nothing was added");
    }

    /// A source with no markup is said plainly, not framed into one scene
    /// holding the whole song.
    #[test]
    fn a_source_with_no_markup_is_refused() {
        let (mut conn, profile_id) = workspace();
        let song_id = work::create(
            &conn,
            &profile_id,
            NewWork {
                kind: "song".into(),
                title: "Harbour lights".into(),
                ..NewWork::default()
            },
        )
        .unwrap()
        .id;
        crate::work::version::create(
            &mut conn,
            &song_id,
            crate::work::version::NewVersion {
                role: "lyrics".into(),
                body: "a few lines
with no markers
"
                .into(),
                label: None,
                meta: None,
                make_current: true,
                parent_version_id: None,
            },
        )
        .unwrap();
        let video_id = video(&conn, &profile_id);
        crate::link::create(
            &conn,
            &profile_id,
            crate::link::NewLink {
                work_id: video_id.clone(),
                source_id: song_id,
                role: None,
                source_version_id: None,
            },
        )
        .unwrap();

        let refused = frame_from_text(&mut conn, &video_id, "lyrics", &[], None);
        assert!(refused.is_err());
        assert_eq!(count(&conn, &video_id).unwrap(), 0, "no half-built board");
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

    /// A board numbered from a list is 1..N, whatever it held before.
    ///
    /// The board here starts out with the disorder `position` deliberately
    /// allows — two scenes numbered 7, nothing numbered 2 — because that is
    /// the state a restored scene and a hand-typed number leave behind, and
    /// it is what this call exists to end. An arithmetic shift would carry
    /// the mess forward; setting the order cannot.
    #[test]
    fn a_renumbering_ends_the_disorder_it_finds() {
        let (mut conn, profile_id) = workspace();
        let work_id = video(&conn, &profile_id);

        let ids: Vec<String> = [7, 1, 7, 4]
            .iter()
            .map(|position| {
                create(
                    &conn,
                    &profile_id,
                    NewScene {
                        work_id: work_id.clone(),
                        position: Some(*position),
                        ..NewScene::default()
                    },
                )
                .unwrap()
                .id
            })
            .collect();

        // The order the person wants: the last scene first, then the rest.
        let wanted = vec![
            ids[3].clone(),
            ids[0].clone(),
            ids[1].clone(),
            ids[2].clone(),
        ];
        let board = renumber(&mut conn, &work_id, &wanted, &now(), None).unwrap();

        assert_eq!(
            board.iter().map(|scene| scene.position).collect::<Vec<_>>(),
            [1, 2, 3, 4],
            "the numbers are set, not nudged: no hole and no twin survives"
        );
        assert_eq!(
            board
                .iter()
                .map(|scene| scene.id.clone())
                .collect::<Vec<_>>(),
            wanted,
            "and they stand in the order asked for"
        );
    }

    /// A list that forgets a scene, or names an outsider, or names one twice,
    /// is refused — each of them would leave the board half-numbered, which
    /// is the state this call exists to remove.
    #[test]
    fn a_renumbering_names_every_scene_exactly_once() {
        let (mut conn, profile_id) = workspace();
        let work_id = video(&conn, &profile_id);
        let other = video(&conn, &profile_id);

        let scene_of = |work: &str| {
            create(
                &conn,
                &profile_id,
                NewScene {
                    work_id: work.to_owned(),
                    ..NewScene::default()
                },
            )
            .unwrap()
            .id
        };
        let first = scene_of(&work_id);
        let second = scene_of(&work_id);
        let stranger = scene_of(&other);

        let numbers = |conn: &Connection| -> Vec<i64> {
            for_work(conn, &work_id)
                .unwrap()
                .iter()
                .map(|scene| scene.position)
                .collect()
        };
        let before = numbers(&conn);

        for wrong in [
            vec![first.clone()],
            vec![first.clone(), second.clone(), stranger.clone()],
            // Every scene named, but one of them twice: the set of
            // names matches the board, and the list is still wrong.
            // This is the one a set comparison alone lets through.
            vec![first.clone(), second.clone(), first.clone()],
            vec![first.clone(), first.clone()],
            vec![first.clone(), stranger.clone()],
        ] {
            assert!(
                renumber(&mut conn, &work_id, &wrong, &now(), None).is_err(),
                "a list of {} names the board wrongly and must be refused",
                wrong.len()
            );
        }

        assert_eq!(
            before,
            numbers(&conn),
            "a refused renumbering changes nothing"
        );
    }

    /// The pictures stay with their scenes across a shift.
    ///
    /// Nothing renames anything: a frame hangs on the scene's id (0021) and
    /// the file in `media/` is named by the asset's id (ADR 0002), so no name
    /// anywhere carries a scene's number. This holds that to be true rather
    /// than assumed — the plan expected a renaming pass here, and there is
    /// none to write.
    #[test]
    fn the_frames_follow_their_scenes_through_a_shift() {
        let (mut conn, profile_id) = workspace();
        let work_id = video(&conn, &profile_id);

        let scenes: Vec<String> = (0..3)
            .map(|_| {
                create(
                    &conn,
                    &profile_id,
                    NewScene {
                        work_id: work_id.clone(),
                        ..NewScene::default()
                    },
                )
                .unwrap()
                .id
            })
            .collect();

        let media = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        let file = outside.path().join("a-picture.png");
        std::fs::write(&file, b"not really a picture").unwrap();

        // The last scene of the board holds the picture; after the shift it
        // is the first, and the picture is still its own.
        let last = scenes[2].clone();
        let frame = crate::scene_frame::attach(
            &conn,
            media.path(),
            &last,
            crate::scene_frame::FRAME,
            &file,
        )
        .unwrap();
        let stored = frame.path.clone();

        let wanted = vec![last.clone(), scenes[0].clone(), scenes[1].clone()];
        renumber(&mut conn, &work_id, &wanted, &now(), None).unwrap();

        let held = crate::scene_frame::for_scene(&conn, &last).unwrap();
        assert_eq!(held.len(), 1, "the scene still holds its picture");
        assert_eq!(
            held[0].path, stored,
            "and the file neither moved nor changed name"
        );
        assert!(
            std::path::Path::new(&stored).exists(),
            "and the bytes are where they were"
        );
        assert_eq!(
            for_work(&conn, &work_id).unwrap()[0].id,
            last,
            "while the scene holding it is now the first"
        );
    }

    /// The spans are left where they are: a renumbering changes what happens
    /// in what order, and the seconds are the person's to settle. Carrying a
    /// span along with its scene would put the last scene at 0:00 because it
    /// is now the first, which is not a board anyone meant.
    #[test]
    fn a_renumbering_leaves_the_spans_alone() {
        let (mut conn, profile_id) = workspace();
        let work_id = video(&conn, &profile_id);

        let ids: Vec<String> = [(0.0, 10.0), (10.0, 20.0)]
            .iter()
            .map(|(starts_at, ends_at)| {
                create(
                    &conn,
                    &profile_id,
                    NewScene {
                        work_id: work_id.clone(),
                        starts_at: Some(*starts_at),
                        ends_at: Some(*ends_at),
                        ..NewScene::default()
                    },
                )
                .unwrap()
                .id
            })
            .collect();

        let board = renumber(
            &mut conn,
            &work_id,
            &[ids[1].clone(), ids[0].clone()],
            &now(),
            None,
        )
        .unwrap();

        assert_eq!(
            (board[0].starts_at, board[0].ends_at),
            (Some(10.0), Some(20.0)),
            "the scene kept the seconds it was given, and only moved"
        );
    }

    /// The order that puts a scene at a number, with the rest closing up.
    ///
    /// The screen's arithmetic for "insert between 7 and 8" and "drag 12 up
    /// to 3", written once. Dragging past either edge means the edge, because
    /// a person who drags past the end means the end.
    #[test]
    fn an_order_puts_a_scene_where_it_was_dropped() {
        let (conn, profile_id) = workspace();
        let work_id = video(&conn, &profile_id);

        let ids: Vec<String> = (0..4)
            .map(|_| {
                create(
                    &conn,
                    &profile_id,
                    NewScene {
                        work_id: work_id.clone(),
                        ..NewScene::default()
                    },
                )
                .unwrap()
                .id
            })
            .collect();
        let board = for_work(&conn, &work_id).unwrap();

        // The last scene to the front, and the first to the back.
        assert_eq!(
            order_moving(&board, &ids[3], 1),
            vec![
                ids[3].clone(),
                ids[0].clone(),
                ids[1].clone(),
                ids[2].clone()
            ]
        );
        assert_eq!(
            order_moving(&board, &ids[0], 4),
            vec![
                ids[1].clone(),
                ids[2].clone(),
                ids[3].clone(),
                ids[0].clone()
            ]
        );
        // Into the middle: the scene lands AT the number asked for, counted
        // in the board as the person sees it.
        assert_eq!(
            order_moving(&board, &ids[3], 2)[1],
            ids[3],
            "it stands second, as asked"
        );
        // Past either edge is the edge, not a refusal and not a hole.
        assert_eq!(order_moving(&board, &ids[1], 99).last(), Some(&ids[1]));
        assert_eq!(order_moving(&board, &ids[1], -3).first(), Some(&ids[1]));
        // Every scene still there, exactly once — the list `renumber` accepts.
        let mut sorted = order_moving(&board, &ids[2], 1);
        sorted.sort();
        let mut all = ids.clone();
        all.sort();
        assert_eq!(sorted, all);
    }
}
