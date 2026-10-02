//! Blocks of words: how the bank is sorted (ADR 0052).
//!
//! The owner keeps words for songs to come and sorts them the way they think
//! of them - "space", "the kitchen", "for the slow one". A block is a row
//! with a name and a place in the order the owner gives it; a word in a
//! block is a row of the pair, so one word stands in as many blocks as it
//! belongs to and leaves none of them when it leaves one.

use rusqlite::{Connection, OptionalExtension, params};
use serde::Serialize;

use super::Term;
use crate::error::{Error, Result};
use crate::minted::Minted;

/// A block of the bank.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, ts_rs::TS)]
pub struct TermBlock {
    pub id: String,
    pub profile_id: String,
    pub name: String,
    /// Where it stands among the blocks, from 0.
    pub position: i64,
    pub created_at: String,
    pub updated_at: String,
}

/// A block with the words in it, in the order they were put there.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, ts_rs::TS)]
pub struct BlockView {
    #[serde(flatten)]
    pub block: TermBlock,
    pub term_ids: Vec<String>,
}

const SELECT_BLOCK: &str =
    "SELECT id, profile_id, name, position, created_at, updated_at FROM term_block";

fn read_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<TermBlock> {
    Ok(TermBlock {
        id: row.get(0)?,
        profile_id: row.get(1)?,
        name: row.get(2)?,
        position: row.get(3)?,
        created_at: row.get(4)?,
        updated_at: row.get(5)?,
    })
}

fn named(name: &str) -> Result<String> {
    let name = name.split_whitespace().collect::<Vec<_>>().join(" ");
    if name.is_empty() {
        return Err(Error::refused("block.needsName"));
    }
    Ok(name)
}

/// A block of a profile already called this, whatever the case: two blocks of
/// one name are one block the person cannot tell apart from the other.
fn find_name(
    conn: &Connection,
    profile_id: &str,
    name: &str,
    except: Option<&str>,
) -> Result<Option<TermBlock>> {
    let wanted = name.to_lowercase();
    Ok(list(conn, profile_id)?
        .into_iter()
        .find(|block| Some(block.id.as_str()) != except && block.name.to_lowercase() == wanted))
}

/// Make a block, last in the order, with the id and moment already decided
/// (ADR 0014).
pub fn create_minted(
    conn: &Connection,
    profile_id: &str,
    name: &str,
    minted: Minted,
) -> Result<TermBlock> {
    let name = named(name)?;
    if let Some(existing) = find_name(conn, profile_id, &name, None)? {
        return Err(Error::refused("block.exists").param("name", existing.name));
    }
    let next: i64 = conn.query_row(
        "SELECT coalesce(max(position) + 1, 0) FROM term_block WHERE profile_id = ?1",
        params![profile_id],
        |row| row.get(0),
    )?;
    conn.execute(
        "INSERT INTO term_block (id, profile_id, name, position, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?5)",
        params![minted.id(), profile_id, name, next, minted.at()],
    )?;
    get(conn, minted.id())?.ok_or_else(|| Error::Internal("the block vanished after insert".into()))
}

pub fn get(conn: &Connection, id: &str) -> Result<Option<TermBlock>> {
    Ok(conn
        .query_row(
            &format!("{SELECT_BLOCK} WHERE id = ?1"),
            params![id],
            read_row,
        )
        .optional()?)
}

/// Every block of a profile, in the owner's order.
pub fn list(conn: &Connection, profile_id: &str) -> Result<Vec<TermBlock>> {
    let mut statement = conn.prepare(&format!(
        "{SELECT_BLOCK} WHERE profile_id = ?1 ORDER BY position, created_at, rowid"
    ))?;
    let rows = statement
        .query_map(params![profile_id], read_row)?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows)
}

/// Every block with the words in it: the bank's left column and what each
/// block holds, in one answer.
pub fn views(conn: &Connection, profile_id: &str) -> Result<Vec<BlockView>> {
    let mut statement = conn.prepare(
        "SELECT block_id, term_id FROM term_block_word WHERE profile_id = ?1
          ORDER BY created_at, rowid",
    )?;
    let pairs = statement
        .query_map(params![profile_id], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(list(conn, profile_id)?
        .into_iter()
        .map(|block| {
            let term_ids = pairs
                .iter()
                .filter(|(owner, _)| *owner == block.id)
                .map(|(_, term)| term.clone())
                .collect();
            BlockView { block, term_ids }
        })
        .collect())
}

/// The block of a profile a person or an agent names: by id, or by name
/// whatever the case.
pub fn find(conn: &Connection, profile_id: &str, named: &str) -> Result<Option<TermBlock>> {
    if let Some(found) = get(conn, named)?.filter(|block| block.profile_id == profile_id) {
        return Ok(Some(found));
    }
    find_name(conn, profile_id, named.trim(), None)
}

/// The words of a block, in the order they were put there.
pub fn words(conn: &Connection, block_id: &str) -> Result<Vec<Term>> {
    let mut statement = conn.prepare(
        "SELECT term_id FROM term_block_word WHERE block_id = ?1 ORDER BY created_at, rowid",
    )?;
    let ids = statement
        .query_map(params![block_id], |row| row.get::<_, String>(0))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    let mut out = Vec::with_capacity(ids.len());
    for id in ids {
        if let Some(term) = super::get(conn, &id)? {
            out.push(term);
        }
    }
    Ok(out)
}

/// Rename a block, with the moment already decided.
pub fn rename_at(conn: &Connection, id: &str, name: &str, at: &str) -> Result<TermBlock> {
    let found = get(conn, id)?.ok_or_else(|| unknown(id))?;
    let name = named(name)?;
    if let Some(existing) = find_name(conn, &found.profile_id, &name, Some(id))? {
        return Err(Error::refused("block.exists").param("name", existing.name));
    }
    if name != found.name {
        conn.execute(
            "UPDATE term_block SET name = ?2, updated_at = ?3 WHERE id = ?1",
            params![id, name, at],
        )?;
    }
    get(conn, id)?.ok_or_else(|| unknown(id))
}

/// Put a block at `index` in the order, the others closing up around it. The
/// order is written whole, 0 to n: the positions are the order, and a gap or
/// a tie would be a second order beside the one shown.
pub fn move_to_at(conn: &Connection, id: &str, index: usize, at: &str) -> Result<Vec<TermBlock>> {
    let found = get(conn, id)?.ok_or_else(|| unknown(id))?;
    let mut order: Vec<TermBlock> = list(conn, &found.profile_id)?
        .into_iter()
        .filter(|block| block.id != id)
        .collect();
    order.insert(index.min(order.len()), found.clone());
    for (position, block) in order.iter().enumerate() {
        let position = i64::try_from(position).unwrap_or(i64::MAX);
        if block.position != position {
            conn.execute(
                "UPDATE term_block SET position = ?2, updated_at = ?3 WHERE id = ?1",
                params![block.id, position, at],
            )?;
        }
    }
    list(conn, &found.profile_id)
}

/// Where a block stands in the order, from 0.
pub fn index_of(conn: &Connection, id: &str) -> Result<usize> {
    let found = get(conn, id)?.ok_or_else(|| unknown(id))?;
    Ok(list(conn, &found.profile_id)?
        .iter()
        .position(|block| block.id == id)
        .unwrap_or(0))
}

/// Put a word in a block. A word already there is not put there twice.
pub fn add_minted(conn: &Connection, block_id: &str, term_id: &str, minted: Minted) -> Result<()> {
    let block = get(conn, block_id)?.ok_or_else(|| unknown(block_id))?;
    let term = super::get(conn, term_id)?.ok_or_else(|| Error::not_found("term", term_id))?;
    if term.profile_id != block.profile_id {
        return Err(Error::not_found("term", term_id));
    }
    conn.execute(
        "INSERT INTO term_block_word (id, profile_id, block_id, term_id, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5) ON CONFLICT (block_id, term_id) DO NOTHING",
        params![
            minted.id(),
            block.profile_id,
            block_id,
            term_id,
            minted.at()
        ],
    )?;
    Ok(())
}

/// Whether a word is in a block.
pub fn holds(conn: &Connection, block_id: &str, term_id: &str) -> Result<bool> {
    Ok(conn.query_row(
        "SELECT EXISTS (SELECT 1 FROM term_block_word WHERE block_id = ?1 AND term_id = ?2)",
        params![block_id, term_id],
        |row| row.get(0),
    )?)
}

/// Take a word out of a block - the word stays in the bank and in its other
/// blocks. Whether it was there.
pub fn remove(conn: &Connection, block_id: &str, term_id: &str) -> Result<bool> {
    Ok(conn.execute(
        "DELETE FROM term_block_word WHERE block_id = ?1 AND term_id = ?2",
        params![block_id, term_id],
    )? > 0)
}

fn unknown(id: &str) -> Error {
    Error::not_found("term_block", id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixtures;
    use crate::register::{self, NewTerm};

    fn block(conn: &Connection, profile_id: &str, name: &str) -> TermBlock {
        create_minted(conn, profile_id, name, Minted::fresh()).unwrap()
    }

    fn word(conn: &Connection, profile_id: &str, word: &str) -> Term {
        register::create(conn, profile_id, NewTerm::banked(word)).unwrap()
    }

    #[test]
    fn blocks_stand_in_the_order_they_were_made_and_keep_a_name_once() {
        let (conn, profile_id) = fixtures::workspace();
        block(&conn, &profile_id, "Space");
        block(&conn, &profile_id, "  the   kitchen ");

        let names: Vec<String> = list(&conn, &profile_id)
            .unwrap()
            .into_iter()
            .map(|block| block.name)
            .collect();
        assert_eq!(names, ["Space", "the kitchen"]);
        let twice = create_minted(&conn, &profile_id, "space", Minted::fresh()).unwrap_err();
        assert_eq!(twice.refusal().map(|r| r.code), Some("block.exists"));
        let blank = create_minted(&conn, &profile_id, "  ", Minted::fresh()).unwrap_err();
        assert_eq!(blank.refusal().map(|r| r.code), Some("block.needsName"));
    }

    #[test]
    fn a_word_stands_in_several_blocks_and_leaves_one_alone() {
        let (conn, profile_id) = fixtures::workspace();
        let space = block(&conn, &profile_id, "Space");
        let slow = block(&conn, &profile_id, "For the slow one");
        let pulsar = word(&conn, &profile_id, "пульсар");
        let nebula = word(&conn, &profile_id, "туманность");

        add_minted(&conn, &space.id, &pulsar.id, Minted::fresh()).unwrap();
        add_minted(&conn, &space.id, &nebula.id, Minted::fresh()).unwrap();
        add_minted(&conn, &slow.id, &pulsar.id, Minted::fresh()).unwrap();
        add_minted(&conn, &slow.id, &pulsar.id, Minted::fresh()).unwrap();

        let held = |id: &str| -> Vec<String> {
            words(&conn, id)
                .unwrap()
                .into_iter()
                .map(|t| t.word)
                .collect()
        };
        assert_eq!(held(&space.id), ["пульсар", "туманность"]);
        assert_eq!(
            held(&slow.id),
            ["пульсар"],
            "put there twice is put there once"
        );

        assert!(remove(&conn, &space.id, &pulsar.id).unwrap());
        assert_eq!(held(&space.id), ["туманность"]);
        assert_eq!(held(&slow.id), ["пульсар"], "the other block keeps it");
        assert!(register::get(&conn, &pulsar.id).unwrap().is_some());
    }

    #[test]
    fn a_block_moves_and_the_others_close_up() {
        let (conn, profile_id) = fixtures::workspace();
        let a = block(&conn, &profile_id, "A");
        block(&conn, &profile_id, "B");
        block(&conn, &profile_id, "C");

        let order = move_to_at(&conn, &a.id, 2, "2026-10-01T00:00:00.000Z").unwrap();
        let names: Vec<(String, i64)> = order.into_iter().map(|b| (b.name, b.position)).collect();
        assert_eq!(
            names,
            [
                ("B".to_owned(), 0),
                ("C".to_owned(), 1),
                ("A".to_owned(), 2)
            ]
        );
        assert_eq!(index_of(&conn, &a.id).unwrap(), 2);
        let first = move_to_at(&conn, &a.id, 0, "2026-10-01T00:00:01.000Z").unwrap();
        assert_eq!(first[0].id, a.id);
    }

    #[test]
    fn a_block_is_found_by_its_id_or_its_name() {
        let (conn, profile_id) = fixtures::workspace();
        let space = block(&conn, &profile_id, "Space");
        assert_eq!(
            find(&conn, &profile_id, &space.id).unwrap(),
            Some(space.clone())
        );
        assert_eq!(find(&conn, &profile_id, " SPACE ").unwrap(), Some(space));
        assert_eq!(find(&conn, &profile_id, "kitchen").unwrap(), None);
    }
}
