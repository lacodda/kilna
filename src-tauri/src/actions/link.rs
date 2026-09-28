//! Gestures on links between works: what a work was made from.

use rusqlite::Connection;

use super::gesture;
use crate::error::{Error, Result};
use crate::journal::Record;
use crate::link::{self, Link, NewLink};

/// Say that a work was made from another, remembering the source's version.
pub fn create(conn: &Connection, new: NewLink) -> Result<Link> {
    gesture(conn, "link.create", |act| {
        act.json("link", &new)?;
        let minted = act.mint();
        let created = link::create_minted(act, act.profile_id(), new, minted)?;
        act.journal(
            Record::new("link.created")
                .param("title", act.title_of(&created.work_id))
                .param("source", created.source_title.clone())
                .about("work", created.work_id.clone()),
        );
        Ok(created)
    })
}

/// Unsay it. The link goes outright, tombstoned by the schema: it is a fact
/// about two works, not a thing with a body worth a drawer in the trash.
pub fn delete(conn: &Connection, id: &str) -> Result<()> {
    gesture(conn, "link.delete", |act| {
        let before = link::get(act, id)?.ok_or_else(|| Error::not_found("link", id))?;
        // The row itself goes into the log, so an undo can put it back under
        // the same id and moment: a link has no drawer in the trash to come
        // back from.
        act.param("id", id);
        act.param(
            "before",
            serde_json::json!({
                "work_id": before.work_id,
                "source_id": before.source_id,
                "role": before.role,
                "source_version_id": before.source_version_id,
                "created_at": before.created_at,
            }),
        );
        link::delete(act, id)?;
        act.journal(
            Record::new("link.removed")
                .param("title", act.title_of(&before.work_id))
                .param("source", before.source_title)
                .about("work", before.work_id),
        );
        Ok(())
    })
}
