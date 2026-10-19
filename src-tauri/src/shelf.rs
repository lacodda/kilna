//! Shelves of phrases: how the owner sorts what they keep (ADR 0064).
//!
//! The owner keeps lines for works to come and sorts them the way they think
//! of them - "space", "the kitchen", the song a handful is gathered for. A
//! shelf is a row with a name and a place in the order the owner gives it; a
//! phrase on a shelf is a row of the pair, so one phrase stands on as many
//! shelves as it belongs to and leaves none of them when it leaves one.
//!
//! The bank of words had these as blocks of terms (ADR 0052); since v0.98 a
//! kept word is a phrase, and its blocks are shelves of notes.

use rusqlite::{Connection, OptionalExtension, params};
use serde::Serialize;

use crate::error::{Error, Result};
use crate::minted::Minted;
use crate::note::Note;

/// A shelf of phrases.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, ts_rs::TS)]
pub struct Shelf {
    pub id: String,
    pub profile_id: String,
    pub name: String,
    /// Where it stands among the shelves, from 0.
    pub position: i64,
    pub created_at: String,
    pub updated_at: String,
}

/// A shelf with the phrases on it, in the order they were put there.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, ts_rs::TS)]
pub struct ShelfView {
    #[serde(flatten)]
    pub shelf: Shelf,
    pub note_ids: Vec<String>,
}

const SELECT_SHELF: &str =
    "SELECT id, profile_id, name, position, created_at, updated_at FROM shelf";

fn read_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Shelf> {
    Ok(Shelf {
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
        return Err(Error::refused("shelf.needsName"));
    }
    Ok(name)
}

/// A shelf of a profile already called this, whatever the case: two shelves
/// of one name are one shelf the person cannot tell apart from the other.
fn find_name(
    conn: &Connection,
    profile_id: &str,
    name: &str,
    except: Option<&str>,
) -> Result<Option<Shelf>> {
    let wanted = crate::words::plain(name);
    Ok(list(conn, profile_id)?.into_iter().find(|shelf| {
        Some(shelf.id.as_str()) != except && crate::words::plain(&shelf.name) == wanted
    }))
}

/// Make a shelf, last in the order, with the id and moment already decided
/// (ADR 0014).
pub fn create_minted(
    conn: &Connection,
    profile_id: &str,
    name: &str,
    minted: Minted,
) -> Result<Shelf> {
    let name = named(name)?;
    if let Some(existing) = find_name(conn, profile_id, &name, None)? {
        return Err(Error::refused("shelf.exists").param("name", existing.name));
    }
    let next: i64 = conn.query_row(
        "SELECT coalesce(max(position) + 1, 0) FROM shelf WHERE profile_id = ?1",
        params![profile_id],
        |row| row.get(0),
    )?;
    conn.execute(
        "INSERT INTO shelf (id, profile_id, name, position, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?5)",
        params![minted.id(), profile_id, name, next, minted.at()],
    )?;
    get(conn, minted.id())?.ok_or_else(|| Error::Internal("the shelf vanished after insert".into()))
}

pub fn get(conn: &Connection, id: &str) -> Result<Option<Shelf>> {
    Ok(conn
        .query_row(
            &format!("{SELECT_SHELF} WHERE id = ?1"),
            params![id],
            read_row,
        )
        .optional()?)
}

/// Every shelf of a profile, in the owner's order.
pub fn list(conn: &Connection, profile_id: &str) -> Result<Vec<Shelf>> {
    let mut statement = conn.prepare(&format!(
        "{SELECT_SHELF} WHERE profile_id = ?1 ORDER BY position, created_at, rowid"
    ))?;
    let rows = statement
        .query_map(params![profile_id], read_row)?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows)
}

/// Every shelf with the phrases on it: the left column of the phrases and
/// what each shelf holds, in one answer.
pub fn views(conn: &Connection, profile_id: &str) -> Result<Vec<ShelfView>> {
    let mut statement = conn.prepare(
        "SELECT shelf_id, note_id FROM shelf_note WHERE profile_id = ?1
          ORDER BY created_at, rowid",
    )?;
    let pairs = statement
        .query_map(params![profile_id], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(list(conn, profile_id)?
        .into_iter()
        .map(|shelf| {
            let note_ids = pairs
                .iter()
                .filter(|(owner, _)| *owner == shelf.id)
                .map(|(_, note)| note.clone())
                .collect();
            ShelfView { shelf, note_ids }
        })
        .collect())
}

/// The shelf of a profile a person or an agent names: by id, or by name
/// whatever the case.
pub fn find(conn: &Connection, profile_id: &str, named: &str) -> Result<Option<Shelf>> {
    if let Some(found) = get(conn, named)?.filter(|shelf| shelf.profile_id == profile_id) {
        return Ok(Some(found));
    }
    find_name(conn, profile_id, named.trim(), None)
}

/// The phrases on a shelf, in the order they were put there.
pub fn notes(conn: &Connection, shelf_id: &str) -> Result<Vec<Note>> {
    let mut statement = conn
        .prepare("SELECT note_id FROM shelf_note WHERE shelf_id = ?1 ORDER BY created_at, rowid")?;
    let ids = statement
        .query_map(params![shelf_id], |row| row.get::<_, String>(0))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    let mut out = Vec::with_capacity(ids.len());
    for id in ids {
        if let Some(note) = crate::note::get(conn, &id)? {
            out.push(note);
        }
    }
    Ok(out)
}

/// The names of the shelves a phrase stands on, in the shelves' order.
pub fn of_note(conn: &Connection, note_id: &str) -> Result<Vec<String>> {
    let mut statement = conn.prepare(
        "SELECT s.name FROM shelf_note p JOIN shelf s ON s.id = p.shelf_id
          WHERE p.note_id = ?1 ORDER BY s.position, s.created_at, s.rowid",
    )?;
    let names = statement
        .query_map(params![note_id], |row| row.get::<_, String>(0))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(names)
}

/// Rename a shelf, with the moment already decided.
pub fn rename_at(conn: &Connection, id: &str, name: &str, at: &str) -> Result<Shelf> {
    let found = get(conn, id)?.ok_or_else(|| unknown(id))?;
    let name = named(name)?;
    if let Some(existing) = find_name(conn, &found.profile_id, &name, Some(id))? {
        return Err(Error::refused("shelf.exists").param("name", existing.name));
    }
    if name != found.name {
        conn.execute(
            "UPDATE shelf SET name = ?2, updated_at = ?3 WHERE id = ?1",
            params![id, name, at],
        )?;
    }
    get(conn, id)?.ok_or_else(|| unknown(id))
}

/// Put a shelf at `index` in the order, the others closing up around it. The
/// order is written whole, 0 to n: the positions are the order, and a gap or
/// a tie would be a second order beside the one shown.
pub fn move_to_at(conn: &Connection, id: &str, index: usize, at: &str) -> Result<Vec<Shelf>> {
    let found = get(conn, id)?.ok_or_else(|| unknown(id))?;
    let mut order: Vec<Shelf> = list(conn, &found.profile_id)?
        .into_iter()
        .filter(|shelf| shelf.id != id)
        .collect();
    order.insert(index.min(order.len()), found.clone());
    for (position, shelf) in order.iter().enumerate() {
        let position = i64::try_from(position).unwrap_or(i64::MAX);
        if shelf.position != position {
            conn.execute(
                "UPDATE shelf SET position = ?2, updated_at = ?3 WHERE id = ?1",
                params![shelf.id, position, at],
            )?;
        }
    }
    list(conn, &found.profile_id)
}

/// Where a shelf stands in the order, from 0.
pub fn index_of(conn: &Connection, id: &str) -> Result<usize> {
    let found = get(conn, id)?.ok_or_else(|| unknown(id))?;
    Ok(list(conn, &found.profile_id)?
        .iter()
        .position(|shelf| shelf.id == id)
        .unwrap_or(0))
}

/// Put a phrase on a shelf. One already there is not put there twice.
pub fn add_minted(conn: &Connection, shelf_id: &str, note_id: &str, minted: Minted) -> Result<()> {
    let shelf = get(conn, shelf_id)?.ok_or_else(|| unknown(shelf_id))?;
    let note = crate::note::get(conn, note_id)?.ok_or_else(|| Error::not_found("note", note_id))?;
    if note.profile_id != shelf.profile_id {
        return Err(Error::not_found("note", note_id));
    }
    conn.execute(
        "INSERT INTO shelf_note (id, profile_id, shelf_id, note_id, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5) ON CONFLICT (shelf_id, note_id) DO NOTHING",
        params![
            minted.id(),
            shelf.profile_id,
            shelf_id,
            note_id,
            minted.at()
        ],
    )?;
    Ok(())
}

/// Whether a phrase is on a shelf.
pub fn holds(conn: &Connection, shelf_id: &str, note_id: &str) -> Result<bool> {
    Ok(conn.query_row(
        "SELECT EXISTS (SELECT 1 FROM shelf_note WHERE shelf_id = ?1 AND note_id = ?2)",
        params![shelf_id, note_id],
        |row| row.get(0),
    )?)
}

/// Take a phrase off a shelf - it stays among the phrases and on its other
/// shelves. Whether it was there.
pub fn remove(conn: &Connection, shelf_id: &str, note_id: &str) -> Result<bool> {
    Ok(conn.execute(
        "DELETE FROM shelf_note WHERE shelf_id = ?1 AND note_id = ?2",
        params![shelf_id, note_id],
    )? > 0)
}

fn unknown(id: &str) -> Error {
    Error::not_found("shelf", id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixtures;
    use crate::note::{self, NewNote};

    fn shelf(conn: &Connection, profile_id: &str, name: &str) -> Shelf {
        create_minted(conn, profile_id, name, Minted::fresh()).unwrap()
    }

    fn phrase(conn: &Connection, profile_id: &str, body: &str) -> Note {
        note::create(
            conn,
            profile_id,
            NewNote {
                body: body.into(),
                kind: Some("phrase".into()),
                ..NewNote::default()
            },
        )
        .unwrap()
    }

    #[test]
    fn shelves_stand_in_the_order_they_were_made_and_keep_a_name_once() {
        let (conn, profile_id) = fixtures::workspace();
        shelf(&conn, &profile_id, "Space");
        shelf(&conn, &profile_id, "  the   kitchen ");

        let names: Vec<String> = list(&conn, &profile_id)
            .unwrap()
            .into_iter()
            .map(|shelf| shelf.name)
            .collect();
        assert_eq!(names, ["Space", "the kitchen"]);
        let twice = create_minted(&conn, &profile_id, "space", Minted::fresh()).unwrap_err();
        assert_eq!(twice.refusal().map(|r| r.code), Some("shelf.exists"));
        let blank = create_minted(&conn, &profile_id, "  ", Minted::fresh()).unwrap_err();
        assert_eq!(blank.refusal().map(|r| r.code), Some("shelf.needsName"));
    }

    /// The case a name is matched in is the person's, not ASCII's: a shelf
    /// called "Космос" is the one "космос" names.
    #[test]
    fn a_name_is_matched_whatever_its_case_in_any_script() {
        let (conn, profile_id) = fixtures::workspace();
        let space = shelf(&conn, &profile_id, "Космос");

        let twice = create_minted(&conn, &profile_id, "космос", Minted::fresh()).unwrap_err();
        assert_eq!(twice.refusal().map(|r| r.code), Some("shelf.exists"));
        assert_eq!(find(&conn, &profile_id, "КОСМОС").unwrap(), Some(space));
    }

    #[test]
    fn a_phrase_stands_on_several_shelves_and_leaves_one_alone() {
        let (conn, profile_id) = fixtures::workspace();
        let space = shelf(&conn, &profile_id, "Space");
        let slow = shelf(&conn, &profile_id, "For the slow one");
        let pulsar = phrase(&conn, &profile_id, "a pulsar keeps time");
        let nebula = phrase(&conn, &profile_id, "nebula");

        add_minted(&conn, &space.id, &pulsar.id, Minted::fresh()).unwrap();
        add_minted(&conn, &space.id, &nebula.id, Minted::fresh()).unwrap();
        add_minted(&conn, &slow.id, &pulsar.id, Minted::fresh()).unwrap();
        add_minted(&conn, &slow.id, &pulsar.id, Minted::fresh()).unwrap();

        let held = |id: &str| -> Vec<String> {
            notes(&conn, id)
                .unwrap()
                .into_iter()
                .map(|n| n.body)
                .collect()
        };
        assert_eq!(held(&space.id), ["a pulsar keeps time", "nebula"]);
        assert_eq!(
            held(&slow.id),
            ["a pulsar keeps time"],
            "put there twice is put there once"
        );
        assert_eq!(
            of_note(&conn, &pulsar.id).unwrap(),
            ["Space", "For the slow one"]
        );

        assert!(remove(&conn, &space.id, &pulsar.id).unwrap());
        assert_eq!(held(&space.id), ["nebula"]);
        assert_eq!(
            held(&slow.id),
            ["a pulsar keeps time"],
            "the other shelf keeps it"
        );
        assert!(note::get(&conn, &pulsar.id).unwrap().is_some());
    }

    #[test]
    fn a_shelf_moves_and_the_others_close_up() {
        let (conn, profile_id) = fixtures::workspace();
        let a = shelf(&conn, &profile_id, "A");
        shelf(&conn, &profile_id, "B");
        shelf(&conn, &profile_id, "C");

        let order = move_to_at(&conn, &a.id, 2, "2026-10-01T00:00:00.000Z").unwrap();
        let names: Vec<(String, i64)> = order.into_iter().map(|s| (s.name, s.position)).collect();
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
    fn a_shelf_is_found_by_its_id_or_its_name() {
        let (conn, profile_id) = fixtures::workspace();
        let space = shelf(&conn, &profile_id, "Space");
        assert_eq!(
            find(&conn, &profile_id, &space.id).unwrap(),
            Some(space.clone())
        );
        assert_eq!(find(&conn, &profile_id, " SPACE ").unwrap(), Some(space));
        assert_eq!(find(&conn, &profile_id, "kitchen").unwrap(), None);
    }
}
