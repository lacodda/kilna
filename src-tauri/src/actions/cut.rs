//! Gestures on the stretches a short is cut from.

use rusqlite::Connection;

use super::gesture;
use crate::cut::{self, Cut, CutPatch, NewCut};
use crate::error::Result;
use crate::journal::Record;

/// Take a stretch of a source into a short.
pub fn create(conn: &Connection, new: NewCut) -> Result<Cut> {
    gesture(conn, "cut.create", |act| {
        act.json("cut", &new)?;
        let minted = act.mint();
        let created = cut::create_minted(act, act.profile_id(), new, minted)?;
        act.journal(
            Record::new("cut.created")
                .param("title", act.title_of(&created.work_id))
                .param("source", created.source_title.clone())
                .about("work", created.work_id.clone()),
        );
        Ok(created)
    })
}

/// Move an end of a stretch, renumber it, or name it.
pub fn update(conn: &Connection, id: &str, patch: CutPatch) -> Result<Cut> {
    gesture(conn, "cut.update", |act| {
        let before = cut::get(act, id)?;
        act.param("id", id);
        act.json("patch", &patch)?;
        act.before(before.as_ref(), &patch)?;
        cut::update(act, id, patch)
    })
}

/// Put a splice in the order given, 1..N, in one change.
///
/// The whole order travels rather than one stretch and a target number: both
/// gestures the screen offers are the same thing said twice, and the list says
/// it once. The order the splice held travels too, so the undo is this same
/// call with it.
pub fn reorder(conn: &Connection, work_id: &str, ids: &[String]) -> Result<Vec<Cut>> {
    gesture(conn, "cut.reorder", |act| {
        let before: Vec<String> = cut::for_work(act, work_id)?
            .into_iter()
            .map(|cut| cut.id)
            .collect();
        act.param("workId", work_id);
        act.json("ids", &ids)?;
        act.json("before", &before)?;
        cut::reorder(act, work_id, ids)
    })
}
