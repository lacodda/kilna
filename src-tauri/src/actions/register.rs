//! Gestures on the register of repeats: a term entered, edited, and the works
//! named as carrying it (ADR 0044) - and on the bank of words it shares a
//! record with: its blocks, their order, and the words put in them (ADR
//! 0052). Throwing a term or a block away is the trash's gesture, like every
//! deletion.

use rusqlite::Connection;

use super::gesture;
use crate::error::{Error, Result};
use crate::register::block::{self, TermBlock};
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

/// Make a block of the bank, last in the order (ADR 0052).
pub fn create_block(conn: &Connection, name: &str) -> Result<TermBlock> {
    gesture(conn, "block.create", |act| {
        act.param("name", name);
        let minted = act.mint();
        block::create_minted(act, act.profile_id(), name, minted)
    })
}

/// Rename a block. The same name again changes nothing and records nothing.
pub fn rename_block(conn: &Connection, id: &str, name: &str) -> Result<TermBlock> {
    gesture(conn, "block.rename", |act| {
        let before = block::get(act, id)?.ok_or_else(|| Error::not_found("term_block", id))?;
        if before.name == name.trim() {
            act.unchanged();
            return Ok(before);
        }
        act.param("id", id);
        act.param("name", name);
        act.param("before", before.name.clone());
        act.stamped();
        block::rename_at(act, id, name, act.at())
    })
}

/// Put a block at `index` in the order. Where it already stands, nothing.
pub fn move_block(conn: &Connection, id: &str, index: usize) -> Result<Vec<TermBlock>> {
    gesture(conn, "block.move", |act| {
        let from = block::index_of(act, id)?;
        let found = block::get(act, id)?.ok_or_else(|| Error::not_found("term_block", id))?;
        if from == index {
            act.unchanged();
            return block::list(act, &found.profile_id);
        }
        act.param("id", id);
        act.param("name", found.name.clone());
        act.param("index", i64::try_from(index).unwrap_or(i64::MAX));
        act.param("from", i64::try_from(from).unwrap_or(0));
        act.stamped();
        block::move_to_at(act, id, index, act.at())
    })
}

/// Put a word in a block. Already there, it changes nothing and records
/// nothing.
pub fn add_to_block(conn: &Connection, block_id: &str, term_id: &str) -> Result<()> {
    gesture(conn, "block.add", |act| {
        if block::holds(act, block_id, term_id)? {
            act.unchanged();
            return Ok(());
        }
        act.param("blockId", block_id);
        act.param("termId", term_id);
        act.param("word", word_of(act, term_id));
        act.param("block", block_name(act, block_id));
        let minted = act.mint();
        block::add_minted(act, block_id, term_id, minted)
    })
}

/// Take a word out of a block. The row goes outright, like a work named for
/// a term: it is a fact about two things. Its id and moment go into the log,
/// so an undo puts back the same row.
pub fn remove_from_block(conn: &Connection, block_id: &str, term_id: &str) -> Result<()> {
    gesture(conn, "block.remove", |act| {
        let row: Option<(String, String)> = act
            .query_row(
                "SELECT id, created_at FROM term_block_word WHERE block_id = ?1 AND term_id = ?2",
                rusqlite::params![block_id, term_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .map(Some)
            .or_else(|error| match error {
                rusqlite::Error::QueryReturnedNoRows => Ok(None),
                other => Err(other),
            })?;
        let Some((id, at)) = row else {
            return Err(Error::not_found(
                "term_block_word",
                format!("{block_id}/{term_id}"),
            ));
        };
        act.param("blockId", block_id);
        act.param("termId", term_id);
        act.param("word", word_of(act, term_id));
        act.param("block", block_name(act, block_id));
        act.param("id", id);
        act.param("createdAt", at);
        block::remove(act, block_id, term_id)?;
        Ok(())
    })
}

fn word_of(conn: &Connection, term_id: &str) -> String {
    register::get(conn, term_id)
        .ok()
        .flatten()
        .map(|term| term.word)
        .unwrap_or_default()
}

fn block_name(conn: &Connection, block_id: &str) -> String {
    block::get(conn, block_id)
        .ok()
        .flatten()
        .map(|block| block.name)
        .unwrap_or_default()
}

/// What putting words in the bank did: each word as it stands now, and how
/// many of them were new to the record.
#[derive(Debug, Clone, serde::Serialize, ts_rs::TS)]
pub struct Banked {
    pub terms: Vec<Term>,
    pub created: usize,
}

/// Put words in the bank, typed in a line or pasted as a list, and - when a
/// block is named - in that block too (ADR 0052).
///
/// One word, one record: a word the record already keeps - a term of the
/// register, a way of singing - is not written a second time; it joins the
/// bank as fresh, unless the bank already holds it, where it stays as it is.
/// The words are gestures as a hand would make them, one after another, in
/// one unit: the whole list lands or none of it.
pub fn bank_words(conn: &Connection, words: &[String], block_id: Option<&str>) -> Result<Banked> {
    crate::db::unit::atomically(conn, |conn| {
        let profile_id = super::active_profile_id(conn)?;
        let mut terms: Vec<Term> = Vec::new();
        let mut created = 0;
        for word in words {
            let word = word.trim();
            if word.is_empty()
                || terms
                    .iter()
                    .any(|t| crate::words::plain(&t.word) == crate::words::plain(word))
            {
                continue;
            }
            let term = match register::find_word(conn, &profile_id, word, None)? {
                Some(found) if found.bank.is_some() => found,
                Some(found) => update(
                    conn,
                    &found.id,
                    TermPatch {
                        bank: Some(Some(register::Bank::Fresh)),
                        ..TermPatch::default()
                    },
                )?,
                None => {
                    created += 1;
                    create(conn, NewTerm::banked(word))?
                }
            };
            if let Some(block_id) = block_id {
                add_to_block(conn, block_id, &term.id)?;
            }
            terms.push(term);
        }
        Ok(Banked { terms, created })
    })
}

/// "I know, I am keeping it" on a word the guard of repeats found in a song
/// (ADR 0054): the finding stays drawn and stops counting. Said twice, it
/// changes nothing and records nothing.
pub fn keep_repeat(conn: &Connection, work_id: &str, word: &str) -> Result<()> {
    gesture(conn, "repeat.keep", |act| {
        if crate::register::guard::is_kept(act, work_id, word)? {
            act.unchanged();
            return Ok(());
        }
        act.param("workId", work_id);
        act.param("word", word.trim().to_lowercase());
        act.param("title", act.title_of(work_id));
        let minted = act.mint();
        crate::register::guard::keep_minted(act, act.profile_id(), work_id, word, minted)
    })
}

/// Take "I am keeping it" back: the finding counts again. The row's id and
/// moment go into the log, so an undo puts back the same row.
pub fn unkeep_repeat(conn: &Connection, work_id: &str, word: &str) -> Result<()> {
    gesture(conn, "repeat.unkeep", |act| {
        let word = word.trim().to_lowercase();
        let row: Option<(String, String)> = act
            .query_row(
                "SELECT id, created_at FROM repeat_kept WHERE work_id = ?1 AND word = ?2",
                rusqlite::params![work_id, word],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .map(Some)
            .or_else(|error| match error {
                rusqlite::Error::QueryReturnedNoRows => Ok(None),
                other => Err(other),
            })?;
        let Some((id, at)) = row else {
            act.unchanged();
            return Ok(());
        };
        act.param("workId", work_id);
        act.param("word", word.clone());
        act.param("title", act.title_of(work_id));
        act.param("id", id);
        act.param("createdAt", at);
        crate::register::guard::unkeep(act, work_id, &word)?;
        Ok(())
    })
}
