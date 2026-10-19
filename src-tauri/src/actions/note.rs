//! Gestures on notes: phrases, notes, lore, references - lines kept from a
//! list, and a note that grows into a work.

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

/// Turn a note into a work it is tied to (ADR 0064).
///
/// One operation for the two rows it touches - the work made, the note tied
/// to it - so an undo takes the whole gesture back. What the note was before
/// is carried, so the undo can put it back: a material note was spent by the
/// work as well (ADR 0045).
pub fn promote(conn: &Connection, id: &str, promotion: note::Promotion) -> Result<note::Promoted> {
    gesture(conn, "note.promote", |act| {
        let before = note::get(act, id)?;
        let work = act.fresh();
        act.param("id", id);
        act.json("promotion", &promotion)?;
        act.param("workId", work.id());
        act.param("title", promotion.title.trim());
        act.param(
            "before",
            serde_json::json!({
                "state": before.as_ref().map(|n| n.state),
                "work_id": before.as_ref().and_then(|n| n.work_id.clone()),
            }),
        );
        act.stamped();

        let promoted = note::promote(act, act.profile_id(), id, promotion, work)?;
        act.journal(
            Record::new("note.promoted")
                .param("title", act.title_of(&promoted.work_id))
                .about("work", promoted.work_id.clone()),
        );
        Ok(promoted)
    })
}

/// What keeping lines did: each line as it stands now, and how many of them
/// were new.
#[derive(Debug, Clone, serde::Serialize, ts_rs::TS)]
pub struct Kept {
    pub notes: Vec<Note>,
    pub created: usize,
}

/// Keep lines typed in a field or pasted as a list, as phrases of `kind` -
/// and, when a shelf is named, on that shelf too (ADR 0064).
///
/// A line already kept in the same words, whatever their case, is not written
/// a second time: it stays as it stands and joins the shelf. The lines are
/// gestures as a hand would make them, one after another, in one unit: the
/// whole list lands or none of it.
pub fn keep_lines(
    conn: &Connection,
    lines: &[String],
    kind: &str,
    shelf_id: Option<&str>,
    tags: &[String],
) -> Result<Kept> {
    crate::db::unit::atomically(conn, |conn| {
        let profile_id = super::active_profile_id(conn)?;
        let mut notes: Vec<Note> = Vec::new();
        let mut created = 0;
        for line in lines {
            let line = line.trim();
            if line.is_empty()
                || notes
                    .iter()
                    .any(|n| crate::words::plain(&n.body) == crate::words::plain(line))
            {
                continue;
            }
            let kept = match crate::lines::find(conn, &profile_id, line)? {
                Some(found) => found,
                None => {
                    created += 1;
                    create(
                        conn,
                        NewNote {
                            body: line.to_owned(),
                            kind: Some(kind.to_owned()),
                            tags: tags.to_vec(),
                            ..NewNote::default()
                        },
                    )?
                }
            };
            if let Some(shelf_id) = shelf_id {
                super::shelf::add(conn, shelf_id, &kept.id)?;
            }
            notes.push(kept);
        }
        Ok(Kept { notes, created })
    })
}

/// Rename a tag on every note that carries it - or, with no new name, take
/// it off them - as one gesture, so one undo takes it back (ADR 0064). How
/// many notes changed.
pub fn retag(conn: &Connection, from: &str, to: Option<&str>) -> Result<usize> {
    gesture(conn, "note.retag", |act| {
        act.param("from", from);
        act.param("to", to.map(str::to_owned));
        act.stamped();
        let before = note::retag_at(act, act.profile_id(), from, to, act.at())?;
        if before.is_empty() {
            act.unchanged();
            return Ok(0);
        }
        act.json(
            "before",
            &before
                .iter()
                .map(|(id, tags)| serde_json::json!({ "id": id, "tags": tags }))
                .collect::<Vec<_>>(),
        )?;
        Ok(before.len())
    })
}
