//! Gestures on the shelves of phrases: a shelf made, renamed, put elsewhere
//! in the order, and the phrases put on it and taken off (ADR 0064). Throwing
//! a shelf away is the trash's gesture, like every deletion.

use rusqlite::Connection;

use super::gesture;
use crate::error::{Error, Result};
use crate::shelf::{self, Shelf};

/// Make a shelf, last in the order.
pub fn create(conn: &Connection, name: &str) -> Result<Shelf> {
    gesture(conn, "shelf.create", |act| {
        act.param("name", name);
        let minted = act.mint();
        shelf::create_minted(act, act.profile_id(), name, minted)
    })
}

/// Rename a shelf. The same name again changes nothing and records nothing.
pub fn rename(conn: &Connection, id: &str, name: &str) -> Result<Shelf> {
    gesture(conn, "shelf.rename", |act| {
        let before = shelf::get(act, id)?.ok_or_else(|| Error::not_found("shelf", id))?;
        if before.name == name.trim() {
            act.unchanged();
            return Ok(before);
        }
        act.param("id", id);
        act.param("name", name);
        act.param("before", before.name.clone());
        act.stamped();
        shelf::rename_at(act, id, name, act.at())
    })
}

/// Put a shelf at `index` in the order. Where it already stands, nothing.
pub fn move_to(conn: &Connection, id: &str, index: usize) -> Result<Vec<Shelf>> {
    gesture(conn, "shelf.move", |act| {
        let from = shelf::index_of(act, id)?;
        let found = shelf::get(act, id)?.ok_or_else(|| Error::not_found("shelf", id))?;
        if from == index {
            act.unchanged();
            return shelf::list(act, &found.profile_id);
        }
        act.param("id", id);
        act.param("name", found.name.clone());
        act.param("index", i64::try_from(index).unwrap_or(i64::MAX));
        act.param("from", i64::try_from(from).unwrap_or(0));
        act.stamped();
        shelf::move_to_at(act, id, index, act.at())
    })
}

/// Put a phrase on a shelf. Already there, it changes nothing and records
/// nothing.
pub fn add(conn: &Connection, shelf_id: &str, note_id: &str) -> Result<()> {
    gesture(conn, "shelf.add", |act| {
        if shelf::holds(act, shelf_id, note_id)? {
            act.unchanged();
            return Ok(());
        }
        act.param("shelfId", shelf_id);
        act.param("noteId", note_id);
        act.param("line", line_of(act, note_id));
        act.param("shelf", shelf_name(act, shelf_id));
        let minted = act.mint();
        shelf::add_minted(act, shelf_id, note_id, minted)
    })
}

/// Take a phrase off a shelf. The row goes outright: it is a fact about two
/// things. Its id and moment go into the log, so an undo puts back the same
/// row.
pub fn remove(conn: &Connection, shelf_id: &str, note_id: &str) -> Result<()> {
    gesture(conn, "shelf.remove", |act| {
        let row: Option<(String, String)> = act
            .query_row(
                "SELECT id, created_at FROM shelf_note WHERE shelf_id = ?1 AND note_id = ?2",
                rusqlite::params![shelf_id, note_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .map(Some)
            .or_else(|error| match error {
                rusqlite::Error::QueryReturnedNoRows => Ok(None),
                other => Err(other),
            })?;
        let Some((id, at)) = row else {
            return Err(Error::not_found(
                "shelf_note",
                format!("{shelf_id}/{note_id}"),
            ));
        };
        act.param("shelfId", shelf_id);
        act.param("noteId", note_id);
        act.param("line", line_of(act, note_id));
        act.param("shelf", shelf_name(act, shelf_id));
        act.param("id", id);
        act.param("createdAt", at);
        shelf::remove(act, shelf_id, note_id)?;
        Ok(())
    })
}

fn line_of(conn: &Connection, note_id: &str) -> String {
    crate::note::get(conn, note_id)
        .ok()
        .flatten()
        .map(|note| note.body)
        .unwrap_or_default()
}

fn shelf_name(conn: &Connection, shelf_id: &str) -> String {
    shelf::get(conn, shelf_id)
        .ok()
        .flatten()
        .map(|shelf| shelf.name)
        .unwrap_or_default()
}
