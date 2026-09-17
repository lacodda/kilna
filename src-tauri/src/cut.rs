//! The stretches of a longer work a short is made of.
//!
//! A short is not its own kind of thing. The `short` kind has stood in the
//! profile since v0.60 with its own axes, roles and door out; what tells a
//! short cut from a finished video apart from one shot for itself is whether
//! it has a donor, and a donor is a [`crate::link`] of role `donor`. One
//! model, the donor being the whole of the difference (ADR 0021).
//!
//! What this module adds is where in the donor. A cut is a stretch of the
//! source's timeline — seconds in, seconds out — and a short holds a list of
//! them in the order they are spliced. A list rather than a pair of numbers
//! because a short is routinely two stretches of one video: the line that
//! lands, then the chorus eight bars later. A pair of columns can hold the
//! first and is silent about the second.
//!
//! The core never opens the file. Cutting belongs to the plugin of v1.10
//! (decision of 2026-09-11: no ffmpeg in the core), and what the plugin is
//! handed is [`shot_list`] — the boundaries beside the path to the donor's
//! video. Everything here is a record of intent.

use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use crate::minted::Minted;
use crate::time::now;

/// One stretch of a source, as the track draws it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Cut {
    pub id: String,
    pub profile_id: String,
    /// The short this stretch is part of.
    pub work_id: String,
    /// The work it is taken out of.
    pub source_id: String,
    /// Seconds on the source's timeline, from its start.
    pub starts_at: f64,
    pub ends_at: f64,
    /// Its place in the splice, from 1.
    pub position: i64,
    /// What the person calls this stretch. Ordinarily nothing: the picture on
    /// the track says more than a name.
    pub label: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    /// The source's title, so a track can be drawn without a second read.
    pub source_title: String,
    /// The source's length in seconds, which is what the track is drawn
    /// against. `None` when the donor has no duration yet — the track then
    /// has no scale and the screen says so rather than inventing one.
    pub source_duration: Option<f64>,
}

impl Cut {
    /// How long this stretch runs.
    pub fn seconds(&self) -> f64 {
        self.ends_at - self.starts_at
    }
}

/// A stretch to take.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewCut {
    pub work_id: String,
    pub source_id: String,
    pub starts_at: f64,
    pub ends_at: f64,
    /// At the end of the splice when omitted.
    #[serde(default)]
    pub position: Option<i64>,
    #[serde(default)]
    pub label: Option<String>,
}

/// What an edit may change. Dragging an end on the track sends one number;
/// the rest stay as they are.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CutPatch {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub starts_at: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ends_at: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub position: Option<i64>,
    #[serde(
        default,
        deserialize_with = "crate::reversal::nullable",
        skip_serializing_if = "Option::is_none"
    )]
    pub label: Option<Option<String>>,
}

const SELECT: &str = "SELECT c.id, c.profile_id, c.work_id, c.source_id, c.starts_at, c.ends_at, \
     c.position, c.label, c.created_at, c.updated_at, w.title, \
     json_extract(w.meta, '$.duration') \
     FROM cut c JOIN work w ON w.id = c.source_id";

/// Take a stretch of a source into a short.
pub fn create(conn: &Connection, profile_id: &str, new: NewCut) -> Result<Cut> {
    create_minted(conn, profile_id, new, Minted::fresh())
}

/// Take a stretch with the id and moment already decided — the seam a replay
/// comes back through, see ADR 0014.
///
/// Both works must be in the profile: a cut across two workspaces is not a
/// thing the track could draw, and the reference would outlive the reason
/// for it.
pub fn create_minted(
    conn: &Connection,
    profile_id: &str,
    new: NewCut,
    minted: Minted,
) -> Result<Cut> {
    check_span(new.starts_at, new.ends_at)?;
    in_profile(conn, profile_id, &new.work_id)?;
    let source = in_profile(conn, profile_id, &new.source_id)?;
    if new.work_id == new.source_id {
        return Err(Error::Other(
            "a work cannot be cut out of itself".to_owned(),
        ));
    }
    check_within(conn, &source, new.ends_at)?;

    // At the end of the splice unless the caller says otherwise: a stretch
    // noticed later is ordinarily a stretch that comes later.
    let position = match new.position {
        Some(position) => position,
        None => next_position(conn, &new.work_id)?,
    };

    conn.execute(
        "INSERT INTO cut (id, profile_id, work_id, source_id, starts_at, ends_at, position, \
         label, created_at, updated_at) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?9)",
        params![
            minted.id(),
            profile_id,
            new.work_id,
            new.source_id,
            new.starts_at,
            new.ends_at,
            position,
            new.label,
            minted.at()
        ],
    )?;

    get(conn, minted.id())?.ok_or_else(|| Error::not_found("cut", minted.id()))
}

/// Move an end, renumber, or name a stretch.
///
/// The span is checked as a whole after the patch is folded in, not field by
/// field: dragging the start past the old end is legal when the end moves in
/// the same gesture, and refusing it halfway would make a legal drag
/// impossible to express.
pub fn update(conn: &Connection, id: &str, patch: CutPatch) -> Result<Cut> {
    let before = get(conn, id)?.ok_or_else(|| Error::not_found("cut", id))?;
    let starts_at = patch.starts_at.unwrap_or(before.starts_at);
    let ends_at = patch.ends_at.unwrap_or(before.ends_at);
    check_span(starts_at, ends_at)?;
    let source = in_profile(conn, &before.profile_id, &before.source_id)?;
    check_within(conn, &source, ends_at)?;

    let label = match &patch.label {
        Some(label) => label.clone(),
        None => before.label.clone(),
    };
    conn.execute(
        "UPDATE cut SET starts_at = ?1, ends_at = ?2, position = ?3, label = ?4, updated_at = ?5 \
         WHERE id = ?6",
        params![
            starts_at,
            ends_at,
            patch.position.unwrap_or(before.position),
            label,
            now(),
            id
        ],
    )?;

    get(conn, id)?.ok_or_else(|| Error::not_found("cut", id))
}

/// Drop a stretch out of the splice.
pub fn delete(conn: &Connection, id: &str) -> Result<()> {
    conn.execute("DELETE FROM cut WHERE id = ?1", params![id])?;
    Ok(())
}

/// Put a short's stretches in the order given, first to last.
///
/// Positions are rewritten from 1 rather than swapped, for the reason
/// [`crate::scene_frame::reorder`] gives: the list the person dragged into
/// shape is the answer, and arithmetic on neighbours is how gaps and
/// duplicates get in.
pub fn reorder(conn: &Connection, work_id: &str, ids: &[String]) -> Result<Vec<Cut>> {
    let known: Vec<String> = for_work(conn, work_id)?
        .into_iter()
        .map(|cut| cut.id)
        .collect();
    if ids.len() != known.len() || !known.iter().all(|id| ids.contains(id)) {
        return Err(Error::Other(
            "reordering a splice names each stretch exactly once".into(),
        ));
    }
    for (index, id) in ids.iter().enumerate() {
        conn.execute(
            "UPDATE cut SET position = ?1, updated_at = ?2 WHERE id = ?3 AND work_id = ?4",
            params![index as i64 + 1, now(), id, work_id],
        )?;
    }
    for_work(conn, work_id)
}

/// One stretch.
pub fn get(conn: &Connection, id: &str) -> Result<Option<Cut>> {
    let mut statement = conn.prepare(&format!("{SELECT} WHERE c.id = ?1"))?;
    Ok(statement.query_row(params![id], read).optional()?)
}

/// A short's stretches, in the order they are spliced.
pub fn for_work(conn: &Connection, work_id: &str) -> Result<Vec<Cut>> {
    let mut statement = conn.prepare(&format!(
        "{SELECT} WHERE c.work_id = ?1 ORDER BY c.position, c.rowid"
    ))?;
    let rows = statement
        .query_map(params![work_id], read)?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows)
}

/// Everything taken out of one work — the question the donor's own card
/// asks: what have I already been cut into.
pub fn from_source(conn: &Connection, source_id: &str) -> Result<Vec<Cut>> {
    let mut statement = conn.prepare(&format!(
        "{SELECT} WHERE c.source_id = ?1 ORDER BY c.starts_at, c.rowid"
    ))?;
    let rows = statement
        .query_map(params![source_id], read)?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows)
}

/// One line of what a cutter is told to do: this stretch, of this file.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Shot {
    pub cut_id: String,
    pub position: i64,
    pub starts_at: f64,
    pub ends_at: f64,
    pub source_id: String,
    pub source_title: String,
    /// The donor's video on disk. `None` when no video is attached to the
    /// donor, which is the one thing that stops a splice from being cuttable.
    pub path: Option<String>,
}

/// What a short is, told to something that can cut video: the stretches in
/// order, each beside the file it comes out of.
///
/// This is the whole of the core's part in making the file. The plugin of
/// v1.10 reads this and runs ffmpeg; the core does not, and this function is
/// the seam between them (decision of 2026-09-11). It is also why `path` is
/// an `Option` rather than an error: a splice written before the donor's
/// video has been rendered is an ordinary half-finished short, and the screen
/// says which line is missing its file rather than refusing to show the list.
///
/// The donor's video is its chosen clip — a `video` asset of the work. Where
/// a donor has several, the most recent is taken: a second render of the
/// same video is a replacement, not a rival.
pub fn shot_list(conn: &Connection, work_id: &str) -> Result<Vec<Shot>> {
    let cuts = for_work(conn, work_id)?;
    let mut shots = Vec::with_capacity(cuts.len());
    for cut in cuts {
        let path: Option<String> = conn
            .query_row(
                "SELECT path FROM asset WHERE work_id = ?1 AND kind = ?2 \
                 ORDER BY created_at DESC, rowid DESC LIMIT 1",
                params![cut.source_id, crate::scene_frame::VIDEO],
                |row| row.get(0),
            )
            .optional()?;
        shots.push(Shot {
            cut_id: cut.id,
            position: cut.position,
            starts_at: cut.starts_at,
            ends_at: cut.ends_at,
            source_id: cut.source_id,
            source_title: cut.source_title,
            path,
        });
    }
    Ok(shots)
}

/// The works a short is spliced from, most recently cut first, without
/// repeats.
///
/// What the calendar's spacing rule reads: two shorts out of one video on
/// consecutive days are the same video twice to anyone watching, whatever
/// the two shorts are called. See [`crate::layout`].
pub fn sources_of(conn: &Connection, work_id: &str) -> Result<Vec<String>> {
    let mut statement = conn.prepare(
        "SELECT source_id FROM cut WHERE work_id = ?1 GROUP BY source_id \
         ORDER BY MIN(position), MIN(rowid)",
    )?;
    let rows = statement
        .query_map(params![work_id], |row| row.get::<_, String>(0))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows)
}

/// Every short's sources in one read, keyed by the short.
///
/// The layout asks this of the whole queue at once. Asking per release would
/// be the shape that made the predecessor's screens slow, and the layout runs
/// this over every day it scans.
pub fn sources_by_work(
    conn: &Connection,
    profile_id: &str,
) -> Result<std::collections::BTreeMap<String, Vec<String>>> {
    let mut statement = conn.prepare(
        "SELECT work_id, source_id FROM cut WHERE profile_id = ?1 \
         GROUP BY work_id, source_id ORDER BY work_id, MIN(position), MIN(rowid)",
    )?;
    let mut map: std::collections::BTreeMap<String, Vec<String>> = Default::default();
    let rows = statement.query_map(params![profile_id], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })?;
    for row in rows {
        let (work_id, source_id) = row?;
        map.entry(work_id).or_default().push(source_id);
    }
    Ok(map)
}

/// A span that ends before it starts is a typo, not a stretch played
/// backwards; a span of no length produces no video and no error anywhere
/// downstream. Both are refused here as well as by the schema, so the person
/// gets a sentence rather than a constraint name.
fn check_span(starts_at: f64, ends_at: f64) -> Result<()> {
    for at in [starts_at, ends_at] {
        if !at.is_finite() {
            return Err(Error::Other("a cut's seconds must be a number".into()));
        }
    }
    if starts_at < 0.0 {
        return Err(Error::Other("a cut starts at zero seconds or later".into()));
    }
    if ends_at <= starts_at {
        return Err(Error::Other("a cut ends after it starts".into()));
    }
    Ok(())
}

/// A stretch that runs past the end of the donor is a stretch of nothing.
///
/// Only checked when the donor has a duration: a video whose length nobody
/// has written down yet is the ordinary state of a work in progress, and
/// refusing to mark a cut in it would make the field mandatory by the back
/// door.
fn check_within(conn: &Connection, source: &crate::work::Work, ends_at: f64) -> Result<()> {
    let _ = conn;
    let Some(duration) = crate::scene::duration_of(source) else {
        return Ok(());
    };
    if ends_at > duration {
        return Err(Error::Other(format!(
            "{} runs {duration} seconds, so a cut cannot end at {ends_at}",
            source.title
        )));
    }
    Ok(())
}

fn in_profile(conn: &Connection, profile_id: &str, work_id: &str) -> Result<crate::work::Work> {
    let work = crate::work::get(conn, work_id)?.ok_or_else(|| Error::not_found("work", work_id))?;
    if work.profile_id != profile_id {
        return Err(Error::Other(format!(
            "{} is not in this workspace",
            work.title
        )));
    }
    Ok(work)
}

fn next_position(conn: &Connection, work_id: &str) -> Result<i64> {
    let last: Option<i64> = conn.query_row(
        "SELECT MAX(position) FROM cut WHERE work_id = ?1",
        params![work_id],
        |row| row.get(0),
    )?;
    Ok(last.unwrap_or(0) + 1)
}

fn read(row: &rusqlite::Row<'_>) -> rusqlite::Result<Cut> {
    Ok(Cut {
        id: row.get(0)?,
        profile_id: row.get(1)?,
        work_id: row.get(2)?,
        source_id: row.get(3)?,
        starts_at: row.get(4)?,
        ends_at: row.get(5)?,
        position: row.get(6)?,
        label: row.get(7)?,
        created_at: row.get(8)?,
        updated_at: row.get(9)?,
        source_title: row.get(10)?,
        source_duration: row.get(11)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::work::{self, NewWork};
    use crate::{db, profile};
    use serde_json::json;

    fn workspace() -> (Connection, String) {
        let conn = db::open_in_memory().unwrap();
        profile::seed(&conn).unwrap();
        let profile_id = profile::active(&conn).unwrap().unwrap().id;
        (conn, profile_id)
    }

    fn a_work(conn: &Connection, profile_id: &str, kind: &str, title: &str) -> String {
        work::create(
            conn,
            profile_id,
            NewWork {
                kind: kind.into(),
                title: title.into(),
                ..NewWork::default()
            },
        )
        .unwrap()
        .id
    }

    /// A donor with a length written down, which is what a track is drawn
    /// against.
    fn a_donor(conn: &Connection, profile_id: &str, title: &str, seconds: f64) -> String {
        let id = a_work(conn, profile_id, "video", title);
        work::update(
            conn,
            &id,
            work::WorkPatch {
                meta: Some(json!({ "duration": seconds }).as_object().unwrap().clone()),
                ..work::WorkPatch::default()
            },
        )
        .unwrap();
        id
    }

    /// The reason this is a table and not two columns: one short is spliced
    /// from two stretches of one video, and the order is the splice.
    #[test]
    fn a_short_is_several_stretches_of_one_video_in_order() {
        let (conn, profile_id) = workspace();
        let donor = a_donor(&conn, &profile_id, "Harbour lights", 210.0);
        let short = a_work(&conn, &profile_id, "short", "Harbour lights — the hook");

        create(
            &conn,
            &profile_id,
            NewCut {
                work_id: short.clone(),
                source_id: donor.clone(),
                starts_at: 48.0,
                ends_at: 61.5,
                position: None,
                label: Some("the line that lands".into()),
            },
        )
        .unwrap();
        create(
            &conn,
            &profile_id,
            NewCut {
                work_id: short.clone(),
                source_id: donor.clone(),
                starts_at: 96.0,
                ends_at: 104.0,
                position: None,
                label: None,
            },
        )
        .unwrap();

        let splice = for_work(&conn, &short).unwrap();
        assert_eq!(splice.len(), 2, "both stretches are kept");
        assert_eq!(splice[0].position, 1);
        assert_eq!(splice[1].position, 2);
        assert_eq!(splice[0].label.as_deref(), Some("the line that lands"));
        assert_eq!(splice[0].seconds(), 13.5);
        assert_eq!(
            splice[0].source_title, "Harbour lights",
            "the track draws the donor without a second read"
        );
        assert_eq!(splice[0].source_duration, Some(210.0));
    }

    /// The donor's card answers from its own end: what has been cut out of me.
    #[test]
    fn a_donor_lists_what_was_taken_out_of_it() {
        let (conn, profile_id) = workspace();
        let donor = a_donor(&conn, &profile_id, "Harbour lights", 210.0);
        let first = a_work(&conn, &profile_id, "short", "The hook");
        let second = a_work(&conn, &profile_id, "short", "The bridge");

        for (work_id, starts_at) in [(&first, 96.0), (&second, 12.0)] {
            create(
                &conn,
                &profile_id,
                NewCut {
                    work_id: work_id.clone(),
                    source_id: donor.clone(),
                    starts_at,
                    ends_at: starts_at + 10.0,
                    position: None,
                    label: None,
                },
            )
            .unwrap();
        }

        let taken = from_source(&conn, &donor).unwrap();
        assert_eq!(taken.len(), 2);
        assert_eq!(
            taken.iter().map(|cut| cut.starts_at).collect::<Vec<_>>(),
            vec![12.0, 96.0],
            "in the order they sit on the donor's own timeline"
        );
    }

    /// A stretch that ends before it starts, or runs no time at all, is a
    /// typo. Mutating `check_span` to let either through must break this.
    #[test]
    fn a_stretch_that_is_not_a_stretch_is_refused() {
        let (conn, profile_id) = workspace();
        let donor = a_donor(&conn, &profile_id, "Harbour lights", 210.0);
        let short = a_work(&conn, &profile_id, "short", "The hook");

        let backwards = create(
            &conn,
            &profile_id,
            NewCut {
                work_id: short.clone(),
                source_id: donor.clone(),
                starts_at: 61.0,
                ends_at: 48.0,
                position: None,
                label: None,
            },
        );
        assert!(backwards.is_err(), "a cut ends after it starts");

        let empty = create(
            &conn,
            &profile_id,
            NewCut {
                work_id: short.clone(),
                source_id: donor.clone(),
                starts_at: 48.0,
                ends_at: 48.0,
                position: None,
                label: None,
            },
        );
        assert!(empty.is_err(), "a cut of no length produces no video");

        assert!(
            for_work(&conn, &short).unwrap().is_empty(),
            "and neither was written"
        );
    }

    /// A stretch past the end of the donor is a stretch of nothing — but only
    /// when the donor's length is known, because not knowing it yet is the
    /// ordinary state of work in progress.
    #[test]
    fn a_stretch_past_the_end_is_refused_only_when_the_end_is_known() {
        let (conn, profile_id) = workspace();
        let timed = a_donor(&conn, &profile_id, "Harbour lights", 210.0);
        let untimed = a_work(&conn, &profile_id, "video", "Still rendering");
        let short = a_work(&conn, &profile_id, "short", "The hook");

        assert!(
            create(
                &conn,
                &profile_id,
                NewCut {
                    work_id: short.clone(),
                    source_id: timed,
                    starts_at: 200.0,
                    ends_at: 240.0,
                    position: None,
                    label: None,
                },
            )
            .is_err(),
            "a 210-second video has nothing at 240"
        );

        assert!(
            create(
                &conn,
                &profile_id,
                NewCut {
                    work_id: short,
                    source_id: untimed,
                    starts_at: 200.0,
                    ends_at: 240.0,
                    position: None,
                    label: None,
                },
            )
            .is_ok(),
            "a donor with no duration yet does not make the field mandatory"
        );
    }

    /// Dragging one end sends one number. The span is judged after the patch
    /// is folded in, so a drag that moves both ends past the old pair is legal
    /// in one gesture.
    #[test]
    fn an_end_moves_on_its_own_and_both_move_together() {
        let (conn, profile_id) = workspace();
        let donor = a_donor(&conn, &profile_id, "Harbour lights", 210.0);
        let short = a_work(&conn, &profile_id, "short", "The hook");
        let cut = create(
            &conn,
            &profile_id,
            NewCut {
                work_id: short,
                source_id: donor,
                starts_at: 48.0,
                ends_at: 61.5,
                position: None,
                label: None,
            },
        )
        .unwrap();

        let moved = update(
            &conn,
            &cut.id,
            CutPatch {
                ends_at: Some(66.0),
                ..CutPatch::default()
            },
        )
        .unwrap();
        assert_eq!(moved.starts_at, 48.0, "the other end stayed");
        assert_eq!(moved.ends_at, 66.0);

        let slid = update(
            &conn,
            &cut.id,
            CutPatch {
                starts_at: Some(90.0),
                ends_at: Some(102.0),
                ..CutPatch::default()
            },
        )
        .unwrap();
        assert_eq!((slid.starts_at, slid.ends_at), (90.0, 102.0));

        assert!(
            update(
                &conn,
                &cut.id,
                CutPatch {
                    starts_at: Some(200.0),
                    ..CutPatch::default()
                },
            )
            .is_err(),
            "dragging the start past the end alone is still a typo"
        );
    }

    /// Cutting a work out of itself is a loop with no bottom, and cutting
    /// across workspaces is a reference that outlives its reason.
    #[test]
    fn a_cut_stays_inside_one_workspace_and_names_two_works() {
        let (conn, profile_id) = workspace();
        let short = a_work(&conn, &profile_id, "short", "The hook");

        assert!(
            create(
                &conn,
                &profile_id,
                NewCut {
                    work_id: short.clone(),
                    source_id: short.clone(),
                    starts_at: 0.0,
                    ends_at: 10.0,
                    position: None,
                    label: None,
                },
            )
            .is_err(),
            "a work cannot be cut out of itself"
        );

        // A second seeded workspace: a cut does not reach into it.
        let others: Vec<_> = profile::list(&conn)
            .unwrap()
            .into_iter()
            .filter(|one| one.id != profile_id)
            .collect();
        let other = others
            .first()
            .expect("the seed ships more than one profile");
        let theirs = work::create(
            &conn,
            &other.id,
            NewWork {
                kind: other.config.work_kinds[0].key.clone(),
                title: "Someone else's".into(),
                ..NewWork::default()
            },
        )
        .unwrap()
        .id;
        assert!(
            create(
                &conn,
                &profile_id,
                NewCut {
                    work_id: short,
                    source_id: theirs,
                    starts_at: 0.0,
                    ends_at: 10.0,
                    position: None,
                    label: None,
                },
            )
            .is_err(),
            "a cut does not reach into another workspace"
        );
    }

    /// Dragging the splice into shape rewrites the order; naming a stretch
    /// twice, or leaving one out, is refused rather than half-applied.
    #[test]
    fn the_splice_is_reordered_by_naming_all_of_it() {
        let (conn, profile_id) = workspace();
        let donor = a_donor(&conn, &profile_id, "Harbour lights", 210.0);
        let short = a_work(&conn, &profile_id, "short", "The hook");
        let mut ids = Vec::new();
        for starts_at in [12.0, 48.0, 96.0] {
            ids.push(
                create(
                    &conn,
                    &profile_id,
                    NewCut {
                        work_id: short.clone(),
                        source_id: donor.clone(),
                        starts_at,
                        ends_at: starts_at + 8.0,
                        position: None,
                        label: None,
                    },
                )
                .unwrap()
                .id,
            );
        }

        let flipped = reorder(
            &conn,
            &short,
            &[ids[2].clone(), ids[0].clone(), ids[1].clone()],
        )
        .unwrap();
        assert_eq!(flipped[0].id, ids[2]);
        assert_eq!(
            flipped.iter().map(|cut| cut.position).collect::<Vec<_>>(),
            vec![1, 2, 3]
        );

        assert!(
            reorder(&conn, &short, &ids[..2]).is_err(),
            "a splice missing a stretch is refused"
        );
        assert!(
            reorder(
                &conn,
                &short,
                &[ids[0].clone(), ids[0].clone(), ids[1].clone()]
            )
            .is_err(),
            "a splice naming one twice is refused"
        );
    }

    /// What a cutter is told to do: the stretches in order, each beside the
    /// donor's file. A donor with no video yet leaves that line without a
    /// path rather than refusing the whole list.
    #[test]
    fn the_shot_list_pairs_every_stretch_with_its_file() {
        let (conn, profile_id) = workspace();
        let donor = a_donor(&conn, &profile_id, "Harbour lights", 210.0);
        let short = a_work(&conn, &profile_id, "short", "The hook");
        create(
            &conn,
            &profile_id,
            NewCut {
                work_id: short.clone(),
                source_id: donor.clone(),
                starts_at: 48.0,
                ends_at: 61.5,
                position: None,
                label: None,
            },
        )
        .unwrap();

        let unrendered = shot_list(&conn, &short).unwrap();
        assert_eq!(unrendered.len(), 1);
        assert_eq!(
            unrendered[0].path, None,
            "a splice written before the donor is rendered is an ordinary half-finished short"
        );

        let media = tempfile::tempdir().unwrap();
        let source = tempfile::tempdir().unwrap();
        let file = source.path().join("harbour.mp4");
        std::fs::write(&file, b"not really an mp4").unwrap();
        crate::asset::attach(
            &conn,
            &profile_id,
            media.path(),
            &file,
            crate::asset::NewAsset {
                work_id: Some(donor),
                kind: Some(crate::scene_frame::VIDEO.into()),
                ..crate::asset::NewAsset::default()
            },
        )
        .unwrap();

        let ready = shot_list(&conn, &short).unwrap();
        assert!(
            ready[0].path.is_some(),
            "once the donor has a video, every line has its file"
        );
        assert_eq!((ready[0].starts_at, ready[0].ends_at), (48.0, 61.5));
    }

    /// What the calendar reads: which videos this short is made of, once each.
    #[test]
    fn a_short_names_its_sources_without_repeats() {
        let (conn, profile_id) = workspace();
        let first = a_donor(&conn, &profile_id, "Harbour lights", 210.0);
        let second = a_donor(&conn, &profile_id, "Winter road", 180.0);
        let short = a_work(&conn, &profile_id, "short", "Two videos");

        for (source, starts_at) in [(&first, 12.0), (&first, 96.0), (&second, 30.0)] {
            create(
                &conn,
                &profile_id,
                NewCut {
                    work_id: short.clone(),
                    source_id: source.clone(),
                    starts_at,
                    ends_at: starts_at + 8.0,
                    position: None,
                    label: None,
                },
            )
            .unwrap();
        }

        assert_eq!(
            sources_of(&conn, &short).unwrap(),
            vec![first.clone(), second.clone()],
            "two stretches of one video name it once"
        );

        let all = sources_by_work(&conn, &profile_id).unwrap();
        assert_eq!(all.get(&short), Some(&vec![first, second]));
    }

    /// A cut is about two works and means nothing without either.
    #[test]
    fn a_cut_does_not_outlive_the_works_it_names() {
        let (conn, profile_id) = workspace();
        let donor = a_donor(&conn, &profile_id, "Harbour lights", 210.0);
        let short = a_work(&conn, &profile_id, "short", "The hook");
        let cut = create(
            &conn,
            &profile_id,
            NewCut {
                work_id: short,
                source_id: donor.clone(),
                starts_at: 48.0,
                ends_at: 61.5,
                position: None,
                label: None,
            },
        )
        .unwrap();

        work::delete(&conn, &donor).unwrap();
        assert!(
            get(&conn, &cut.id).unwrap().is_none(),
            "the stretch goes with the video it was taken from"
        );
    }
}
