//! Gestures on notes: ideas, lore, references - and an idea that grows into a
//! work.

use rusqlite::Connection;

use super::gesture;
use crate::error::Result;
use crate::journal::Record;
use crate::note::{self, NewNote, Note, NotePatch};

pub fn create(conn: &Connection, new: NewNote) -> Result<Note> {
    gesture(conn, "note.create", |act| {
        act.json("note", &new)?;
        let minted = act.mint();
        note::create_minted(act, act.profile_id(), new, minted)
    })
}

pub fn update(conn: &Connection, id: &str, patch: NotePatch) -> Result<Note> {
    gesture(conn, "note.update", |act| {
        let before = note::get(act, id)?;
        act.param("id", id);
        act.json("patch", &patch)?;
        act.before(before.as_ref(), &patch)?;
        act.stamped();
        note::update_at(act, id, patch, act.at())
    })
}

/// Turn a note into a work, its body the work's first version.
///
/// One operation for the three rows it touches — the work, its version, the
/// note moved to the trash — so an undo takes the whole gesture back rather
/// than restoring the note beside a work that still holds its text.
pub fn promote(conn: &Connection, id: &str, promotion: note::Promotion) -> Result<note::Promoted> {
    gesture(conn, "note.promote", |act| {
        let ids = note::PromotionIds {
            work: act.fresh(),
            version: act.fresh(),
            deletion: act.fresh(),
        };
        act.param("id", id);
        act.json("promotion", &promotion)?;
        act.param("workId", ids.work.id());
        act.param("versionId", ids.version.id());
        act.param("entryId", ids.deletion.id());
        act.param("title", promotion.title.trim());
        act.stamped();

        let promoted = note::promote(act, act.profile_id(), id, promotion, &ids)?;
        act.journal(
            Record::new("note.promoted")
                .param("title", act.title_of(&promoted.work_id))
                .about("work", promoted.work_id.clone()),
        );
        Ok(promoted)
    })
}
