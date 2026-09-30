//! Gestures on the register of repeats: a term entered, edited, and the works
//! named as carrying it (ADR 0044). Throwing a term away is the trash's
//! gesture, like every deletion.

use rusqlite::Connection;

use super::gesture;
use crate::error::{Error, Result};
use crate::register::{self, NewTerm, Term, TermPatch};

pub fn create(conn: &Connection, new: NewTerm) -> Result<Term> {
    gesture(conn, "term.create", |act| {
        act.json("term", &new)?;
        let minted = act.mint();
        register::create_minted(act, act.profile_id(), new, minted)
    })
}

pub fn update(conn: &Connection, id: &str, patch: TermPatch) -> Result<Term> {
    gesture(conn, "term.update", |act| {
        let before = register::get(act, id)?;
        act.param("id", id);
        act.json("patch", &patch)?;
        act.before(before.as_ref(), &patch)?;
        act.stamped();
        register::update_at(act, id, patch, act.at())
    })
}

/// Name a work as carrying a term - how a meaning is tied to the works it is
/// in. Naming it twice changes nothing and records nothing.
pub fn link(conn: &Connection, term_id: &str, work_id: &str) -> Result<()> {
    gesture(conn, "term.link", |act| {
        if linked(act, term_id, work_id)? {
            act.unchanged();
            return Ok(());
        }
        act.param("termId", term_id);
        act.param("workId", work_id);
        act.param("title", act.title_of(work_id));
        let minted = act.mint();
        register::link_minted(act, term_id, work_id, minted)
    })
}

/// Let go of a work named as carrying a term. The row goes outright, like a
/// link between works: it is a fact about two things, not a thing with a body.
pub fn unlink(conn: &Connection, term_id: &str, work_id: &str) -> Result<()> {
    gesture(conn, "term.unlink", |act| {
        let created_at: Option<(String, String)> = act
            .query_row(
                "SELECT id, created_at FROM term_work WHERE term_id = ?1 AND work_id = ?2",
                rusqlite::params![term_id, work_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .map(Some)
            .or_else(|error| match error {
                rusqlite::Error::QueryReturnedNoRows => Ok(None),
                other => Err(other),
            })?;
        let Some((id, at)) = created_at else {
            return Err(Error::not_found(
                "term_work",
                format!("{term_id}/{work_id}"),
            ));
        };
        // The row's own id and moment go into the log, so an undo puts back
        // the same row rather than a new one that looks like it.
        act.param("termId", term_id);
        act.param("workId", work_id);
        act.param("title", act.title_of(work_id));
        act.param("id", id);
        act.param("createdAt", at);
        register::unlink(act, term_id, work_id)?;
        Ok(())
    })
}

fn linked(conn: &Connection, term_id: &str, work_id: &str) -> Result<bool> {
    Ok(conn.query_row(
        "SELECT EXISTS (SELECT 1 FROM term_work WHERE term_id = ?1 AND work_id = ?2)",
        rusqlite::params![term_id, work_id],
        |row| row.get(0),
    )?)
}
