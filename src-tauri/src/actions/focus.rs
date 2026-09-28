//! Gestures on the focus board: dismissed findings and the notes pinned to
//! the board.

use rusqlite::Connection;

use super::gesture;
use crate::error::Result;
use crate::focus::{self, Dismissal, DismissalKey, FocusNote, FocusNotePatch, NewFocusNote};

pub fn dismiss(conn: &Connection, key: &DismissalKey) -> Result<Dismissal> {
    gesture(conn, "finding.dismiss", |act| {
        act.json("key", key)?;
        act.stamped();
        focus::dismiss_at(act, act.profile_id(), key, act.at())
    })
}

pub fn restore(conn: &Connection, key: &DismissalKey) -> Result<()> {
    gesture(conn, "finding.restore", |act| {
        act.json("key", key)?;
        focus::restore(act, act.profile_id(), key)
    })
}

pub fn create_note(conn: &Connection, new: NewFocusNote) -> Result<FocusNote> {
    gesture(conn, "focusNote.create", |act| {
        act.json("note", &new)?;
        let minted = act.mint();
        focus::add_note_minted(act, act.profile_id(), new, minted)
    })
}

pub fn update_note(conn: &Connection, id: &str, patch: FocusNotePatch) -> Result<FocusNote> {
    gesture(conn, "focusNote.update", |act| {
        let before = focus::get_note(act, id)?;
        act.param("id", id);
        act.json("patch", &patch)?;
        act.before(before.as_ref(), &patch)?;
        act.stamped();
        focus::update_note_at(act, id, patch, act.at())
    })
}

pub fn reorder_notes(conn: &Connection, order: &[String]) -> Result<()> {
    gesture(conn, "focusNote.reorder", |act| {
        act.json("order", &order)?;
        focus::reorder_notes(act, act.profile_id(), order)
    })
}

/// Rub a board note out.
///
/// It does not go through the trash the way a work or an idea does: those are
/// content, and losing one by mistake costs the person something. A board note
/// is a line on a surface — restoring it would be a drawer of crossed-out
/// reminders nobody opens.
pub fn delete_note(conn: &Connection, id: &str) -> Result<()> {
    gesture(conn, "focusNote.delete", |act| {
        act.param("id", id);
        focus::delete_note(act, id)
    })
}
