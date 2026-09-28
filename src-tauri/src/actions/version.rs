//! Gestures on a work's versions: a new draft, and an edit of one.

use rusqlite::Connection;

use super::gesture;
use crate::error::Result;
use crate::journal::Record;
use crate::work::version::{self, NewVersion, Version};

/// Add a version to a work.
///
/// Journalled under the work rather than the version: the History tab people
/// want is the work's, and a version's own tab would hold one line saying it
/// was made.
pub fn create(conn: &Connection, work_id: &str, new: NewVersion) -> Result<Version> {
    gesture(conn, "version.create", |act| {
        act.param("workId", work_id);
        act.json("version", &new)?;
        let minted = act.mint();
        let created = version::create_minted(act, work_id, new, minted)?;
        act.journal(
            Record::new("version.created")
                .param("role", created.role.clone())
                .param("revision", created.revision)
                .param("title", act.title_of(work_id))
                .about("work", work_id.to_owned()),
        );
        Ok(created)
    })
}

/// Rewrite a version's body in place.
///
/// The body before travels whole, so an undo has something to put back. Not a
/// diff: a body is one field, and a diff would be a second way of storing
/// text (ADR 0002). Nothing is journalled - the session's edits are one change
/// to the person, and the version's creation already made its line.
pub fn edit(conn: &Connection, id: &str, body: &str) -> Result<Version> {
    gesture(conn, "version.edit", |act| {
        let before = version::get(act, id)?
            .map(|found| found.body)
            .unwrap_or_default();
        act.param("id", id);
        act.param("body", body);
        act.param("before", before);
        act.stamped();
        version::update_body_at(act, id, body, act.at())
    })
}
