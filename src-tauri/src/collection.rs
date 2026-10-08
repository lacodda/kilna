use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::db::unit::atomically;
use crate::error::{Error, Result};
use crate::minted::Minted;
use crate::time::now;

/// An album, a book, a season. One level deep on purpose: a collection never
/// contains a collection. Nesting buys arbitrary depth and costs every screen
/// a tree — see ADR 0001.
#[derive(Debug, Clone, Serialize, ts_rs::TS)]
pub struct Collection {
    pub id: String,
    pub profile_id: String,
    pub kind: String,
    pub title: String,
    pub description: Option<String>,
    pub position: i64,
    pub meta: Map<String, Value>,
    /// How many works it is meant to hold when finished — twelve tracks, thirty
    /// chapters. Read beside `works` it says how far along the whole is.
    pub target_size: Option<i64>,
    /// The day it is meant to be done by, as a date.
    pub due_on: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    /// The works it holds, in their order: track one first. The order and
    /// the count are one fact, so the collection carries the list rather than
    /// a number beside it - a screen that drew the count from one answer and
    /// the order from another could show twelve and list eleven.
    pub work_ids: Vec<String>,
}

/// Where a work stood: in which collection, and at which place in it.
///
/// What a change to a collection's contents records about every work it
/// touches, so that taking the change back puts each one where it was - in
/// the collection it was taken from, at its old place - rather than only out
/// of the one it was put in.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Placement {
    pub id: String,
    pub collection_id: Option<String>,
    pub position: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
#[ts(optional_fields)]
pub struct NewCollection {
    pub kind: String,
    pub title: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub meta: Option<Map<String, Value>>,
    #[serde(default)]
    pub target_size: Option<i64>,
    #[serde(default)]
    pub due_on: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, ts_rs::TS)]
pub struct CollectionPatch {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(
        default,
        deserialize_with = "crate::reversal::nullable",
        skip_serializing_if = "Option::is_none"
    )]
    pub description: Option<Option<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub meta: Option<Map<String, Value>>,
    #[serde(
        default,
        deserialize_with = "crate::reversal::nullable",
        skip_serializing_if = "Option::is_none"
    )]
    pub target_size: Option<Option<i64>>,
    #[serde(
        default,
        deserialize_with = "crate::reversal::nullable",
        skip_serializing_if = "Option::is_none"
    )]
    pub due_on: Option<Option<String>>,
}

const SELECT_COLLECTION: &str = "SELECT c.id, c.profile_id, c.kind, c.title, c.description, c.position, c.meta, \
     c.created_at, c.updated_at, c.target_size, c.due_on \
     FROM collection c";

/// The works in a collection, in its order.
///
/// Ties - two works a patch put at the same place before v0.92 - are broken
/// by when the work was made, then by the row, never by the id: an id is a
/// UUID, and ordering by one is ordering at random.
pub fn members(conn: &Connection, id: &str) -> Result<Vec<String>> {
    let mut statement = conn.prepare(
        "SELECT id FROM work WHERE collection_id = ?1 ORDER BY position, created_at, rowid",
    )?;
    let ids = statement
        .query_map(params![id], |row| row.get(0))?
        .collect::<rusqlite::Result<Vec<String>>>()?;
    Ok(ids)
}

/// Where each of these works stands now. A work the profile does not hold is
/// left out: it is not there to place, and a caller that asked about it is
/// told so by its absence.
pub fn placements<'a>(
    conn: &Connection,
    profile_id: &str,
    work_ids: impl IntoIterator<Item = &'a String>,
) -> Result<Vec<Placement>> {
    let mut statement = conn.prepare(
        "SELECT id, collection_id, position FROM work WHERE id = ?1 AND profile_id = ?2",
    )?;
    let mut found = Vec::new();
    for work_id in work_ids {
        let placement = statement
            .query_row(params![work_id, profile_id], |row| {
                Ok(Placement {
                    id: row.get(0)?,
                    collection_id: row.get(1)?,
                    position: row.get(2)?,
                })
            })
            .optional()?;
        found.extend(placement);
    }
    Ok(found)
}

/// Put every work back where a [`Placement`] says it stood. One unit: works
/// half put back are an arrangement nobody made.
pub fn restore_at(conn: &Connection, placements: &[Placement], at: &str) -> Result<()> {
    atomically(conn, |tx| {
        for placement in placements {
            tx.execute(
                "UPDATE work SET collection_id = ?1, position = ?2, updated_at = ?3 WHERE id = ?4",
                params![
                    placement.collection_id,
                    placement.position,
                    at,
                    placement.id
                ],
            )?;
        }
        Ok(())
    })
}

/// The place after the last work of a collection.
pub fn next_position(conn: &Connection, id: &str) -> Result<i64> {
    Ok(conn.query_row(
        "SELECT coalesce(max(position), -1) + 1 FROM work WHERE collection_id = ?1",
        params![id],
        |row| row.get(0),
    )?)
}

/// Refuse a goal that could not be read back as one.
fn check_goal(target_size: Option<i64>, due_on: Option<&str>) -> Result<()> {
    if let Some(size) = target_size
        && size < 1
    {
        return Err(Error::refused("collection.badTargetSize").param("size", size));
    }
    if let Some(day) = due_on
        && !crate::time::is_date(day)
    {
        return Err(Error::refused("collection.badDueDate").param("value", day));
    }
    Ok(())
}

pub fn create(conn: &Connection, profile_id: &str, new: NewCollection) -> Result<Collection> {
    create_minted(conn, profile_id, new, Minted::fresh())
}

/// Create a collection with the id and timestamp already decided.
///
/// The seam a replay comes back through: live, `create` mints them; replaying,
/// the log supplies what the first run generated, so the collection lands
/// under the id everything else already names. See ADR 0014.
pub fn create_minted(
    conn: &Connection,
    profile_id: &str,
    new: NewCollection,
    minted: Minted,
) -> Result<Collection> {
    let id = minted.id().to_owned();
    let timestamp = minted.at().to_owned();

    let position: i64 = conn.query_row(
        "SELECT coalesce(max(position), -1) + 1 FROM collection WHERE profile_id = ?1",
        params![profile_id],
        |row| row.get(0),
    )?;

    check_goal(new.target_size, new.due_on.as_deref())?;
    conn.execute(
        "INSERT INTO collection (id, profile_id, kind, title, description, position, meta, created_at, updated_at,
                                 target_size, due_on)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?8, ?9, ?10)",
        params![
            id,
            profile_id,
            new.kind,
            new.title,
            new.description,
            position,
            Value::Object(new.meta.unwrap_or_default()).to_string(),
            timestamp,
            new.target_size,
            new.due_on,
        ],
    )?;

    get(conn, &id)?.ok_or_else(|| Error::Internal("the collection vanished after insert".into()))
}

pub fn get(conn: &Connection, id: &str) -> Result<Option<Collection>> {
    let raw = conn
        .query_row(
            &format!("{SELECT_COLLECTION} WHERE c.id = ?1"),
            params![id],
            read_row,
        )
        .optional()?;

    raw.map(|raw| raw.into_collection(conn)).transpose()
}

pub fn list(conn: &Connection, profile_id: &str) -> Result<Vec<Collection>> {
    let mut statement = conn.prepare(&format!(
        "{SELECT_COLLECTION} WHERE c.profile_id = ?1 ORDER BY c.position, c.title"
    ))?;
    let raw = statement
        .query_map(params![profile_id], read_row)?
        .collect::<rusqlite::Result<Vec<_>>>()?;

    raw.into_iter()
        .map(|raw| raw.into_collection(conn))
        .collect()
}

pub fn update(conn: &Connection, id: &str, patch: CollectionPatch) -> Result<Collection> {
    update_at(conn, id, patch, &now())
}

/// Apply a patch with the change's timestamp already decided.
///
/// The seam a replay comes back through: live, `update` stamps `now()`;
/// replaying, the log supplies the moment the first run recorded. See ADR 0014.
pub fn update_at(
    conn: &Connection,
    id: &str,
    patch: CollectionPatch,
    at: &str,
) -> Result<Collection> {
    let mut assignments: Vec<String> = Vec::new();
    let mut values: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

    fn set(
        assignments: &mut Vec<String>,
        values: &mut Vec<Box<dyn rusqlite::ToSql>>,
        column: &str,
        value: Box<dyn rusqlite::ToSql>,
    ) {
        values.push(value);
        assignments.push(format!("{column} = ?{}", values.len()));
    }

    if let Some(kind) = patch.kind {
        set(&mut assignments, &mut values, "kind", Box::new(kind));
    }
    if let Some(title) = patch.title {
        set(&mut assignments, &mut values, "title", Box::new(title));
    }
    if let Some(description) = patch.description {
        set(
            &mut assignments,
            &mut values,
            "description",
            Box::new(description),
        );
    }
    if let Some(meta) = patch.meta {
        set(
            &mut assignments,
            &mut values,
            "meta",
            Box::new(Value::Object(meta).to_string()),
        );
    }
    check_goal(
        patch.target_size.flatten(),
        patch.due_on.as_ref().and_then(|d| d.as_deref()),
    )?;
    if let Some(target_size) = patch.target_size {
        set(
            &mut assignments,
            &mut values,
            "target_size",
            Box::new(target_size),
        );
    }
    if let Some(due_on) = patch.due_on {
        set(&mut assignments, &mut values, "due_on", Box::new(due_on));
    }

    if assignments.is_empty() {
        return get(conn, id)?.ok_or_else(|| unknown(id));
    }

    set(
        &mut assignments,
        &mut values,
        "updated_at",
        Box::new(at.to_owned()),
    );
    values.push(Box::new(id.to_owned()));

    let sql = format!(
        "UPDATE collection SET {} WHERE id = ?{}",
        assignments.join(", "),
        values.len()
    );
    let params = rusqlite::params_from_iter(values.iter().map(AsRef::as_ref));

    if conn.execute(&sql, params)? == 0 {
        return Err(unknown(id));
    }

    get(conn, id)?.ok_or_else(|| unknown(id))
}

/// Delete a collection. Its works survive and become loose — a container going
/// away must not take its contents with it.
pub fn delete(conn: &Connection, id: &str) -> Result<()> {
    if conn.execute("DELETE FROM collection WHERE id = ?1", params![id])? == 0 {
        return Err(unknown(id));
    }
    Ok(())
}

/// Put works in a collection in the given order. Works not listed are removed
/// from it.
pub fn set_contents(conn: &Connection, id: &str, work_ids: &[String]) -> Result<()> {
    set_contents_at(conn, id, work_ids, &now())
}

/// Set a collection's contents with the change's timestamp already decided.
///
/// The seam a replay comes back through: live, `set_contents` stamps
/// `now()`; replaying, the log supplies the moment the first run recorded.
/// See ADR 0014.
///
/// One unit: contents half set are contents nobody chose.
pub fn set_contents_at(conn: &Connection, id: &str, work_ids: &[String], at: &str) -> Result<()> {
    let exists: bool = conn
        .query_row(
            "SELECT 1 FROM collection WHERE id = ?1",
            params![id],
            |_| Ok(true),
        )
        .optional()?
        .unwrap_or(false);
    if !exists {
        return Err(unknown(id));
    }

    atomically(conn, |tx| {
        tx.execute(
            "UPDATE work SET collection_id = NULL, updated_at = ?2 WHERE collection_id = ?1",
            params![id, at],
        )?;

        for (position, work_id) in work_ids.iter().enumerate() {
            tx.execute(
                "UPDATE work SET collection_id = ?1, position = ?2, updated_at = ?3 WHERE id = ?4",
                params![id, position as i64, at, work_id],
            )?;
        }
        Ok(())
    })
}

/// Put works at the end of a collection, in the order given, taking each out
/// of whatever collection held it: a work belongs to one at most (ADR 0001).
///
/// The caller decides which works go in - one already in this collection is
/// left out by the action, which says so, rather than sent to the end here.
pub fn append_at(conn: &Connection, id: &str, work_ids: &[String], at: &str) -> Result<()> {
    if get(conn, id)?.is_none() {
        return Err(unknown(id));
    }

    atomically(conn, |tx| {
        let first = next_position(tx, id)?;
        for (position, work_id) in (first..).zip(work_ids) {
            tx.execute(
                "UPDATE work SET collection_id = ?1, position = ?2, updated_at = ?3 WHERE id = ?4",
                params![id, position, at, work_id],
            )?;
        }
        Ok(())
    })
}

fn unknown(id: &str) -> Error {
    Error::not_found("collection", id)
}

struct RawCollection {
    id: String,
    profile_id: String,
    kind: String,
    title: String,
    description: Option<String>,
    position: i64,
    meta: String,
    created_at: String,
    updated_at: String,
    target_size: Option<i64>,
    due_on: Option<String>,
}

fn read_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<RawCollection> {
    Ok(RawCollection {
        id: row.get(0)?,
        profile_id: row.get(1)?,
        kind: row.get(2)?,
        title: row.get(3)?,
        description: row.get(4)?,
        position: row.get(5)?,
        meta: row.get(6)?,
        created_at: row.get(7)?,
        updated_at: row.get(8)?,
        target_size: row.get(9)?,
        due_on: row.get(10)?,
    })
}

impl RawCollection {
    fn into_collection(self, conn: &Connection) -> Result<Collection> {
        Ok(Collection {
            work_ids: members(conn, &self.id)?,
            meta: serde_json::from_str(&self.meta)?,
            id: self.id,
            profile_id: self.profile_id,
            kind: self.kind,
            title: self.title,
            description: self.description,
            position: self.position,
            target_size: self.target_size,
            due_on: self.due_on,
            created_at: self.created_at,
            updated_at: self.updated_at,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixtures;
    use crate::work::{self, WorkFilter};

    fn album(conn: &Connection, profile_id: &str, title: &str) -> Collection {
        create(
            conn,
            profile_id,
            NewCollection {
                kind: "album".into(),
                title: title.into(),
                description: None,
                meta: None,
                target_size: None,
                due_on: None,
            },
        )
        .unwrap()
    }

    fn a_work(conn: &Connection, profile_id: &str, title: &str) -> String {
        fixtures::song(conn, profile_id, title).id
    }

    #[test]
    fn a_new_collection_is_empty_and_positioned_last() {
        let (conn, profile_id) = fixtures::workspace();

        let first = album(&conn, &profile_id, "First");
        let second = album(&conn, &profile_id, "Second");

        assert!(first.work_ids.is_empty());
        assert_eq!(first.position, 0);
        assert_eq!(second.position, 1);
    }

    #[test]
    fn set_contents_orders_the_works_and_counts_them() {
        let (conn, profile_id) = fixtures::workspace();
        let collection = album(&conn, &profile_id, "Album");
        let one = a_work(&conn, &profile_id, "One");
        let two = a_work(&conn, &profile_id, "Two");

        set_contents(&conn, &collection.id, &[two.clone(), one.clone()]).unwrap();

        let reloaded = get(&conn, &collection.id).unwrap().unwrap();
        assert_eq!(
            reloaded.work_ids,
            vec![two.clone(), one.clone()],
            "the collection lists its works in the order given"
        );

        let inside = work::list(
            &conn,
            &profile_id,
            &WorkFilter {
                collection_id: Some(collection.id.clone()),
                ..Default::default()
            },
        )
        .unwrap();
        let by_position: Vec<_> = {
            let mut sorted = inside.clone();
            sorted.sort_by_key(|work| work.position);
            sorted.into_iter().map(|work| work.id).collect()
        };
        assert_eq!(by_position, vec![two, one], "the given order is the order");
    }

    #[test]
    fn set_contents_removes_works_left_out() {
        let (conn, profile_id) = fixtures::workspace();
        let collection = album(&conn, &profile_id, "Album");
        let stays = a_work(&conn, &profile_id, "Stays");
        let leaves = a_work(&conn, &profile_id, "Leaves");
        set_contents(&conn, &collection.id, &[stays.clone(), leaves.clone()]).unwrap();

        set_contents(&conn, &collection.id, std::slice::from_ref(&stays)).unwrap();

        assert_eq!(
            get(&conn, &collection.id).unwrap().unwrap().work_ids,
            vec![stays]
        );
        let loose = work::get(&conn, &leaves).unwrap().unwrap();
        assert!(loose.collection_id.is_none(), "the work survives, loose");
    }

    #[test]
    fn deleting_a_collection_leaves_its_works_alone() {
        let (conn, profile_id) = fixtures::workspace();
        let collection = album(&conn, &profile_id, "Album");
        let work_id = a_work(&conn, &profile_id, "Inside");
        set_contents(&conn, &collection.id, std::slice::from_ref(&work_id)).unwrap();

        delete(&conn, &collection.id).unwrap();

        let survivor = work::get(&conn, &work_id).unwrap().unwrap();
        assert!(survivor.collection_id.is_none());
    }

    #[test]
    fn collections_are_scoped_to_a_profile() {
        let (conn, profile_id) = fixtures::workspace();
        album(&conn, &profile_id, "Mine");
        conn.execute(
            "INSERT INTO profile (id, key, name, config, is_active, is_builtin, created_at, updated_at)
             SELECT 'other', 'other', 'Other', config, 0, 0, created_at, updated_at FROM profile LIMIT 1",
            [],
        )
        .unwrap();
        album(&conn, "other", "Theirs");

        let mine = list(&conn, &profile_id).unwrap();

        assert_eq!(mine.len(), 1);
        assert_eq!(mine[0].title, "Mine");
    }

    #[test]
    fn appended_works_follow_the_last_one_and_leave_their_old_collection() {
        let (conn, profile_id) = fixtures::workspace();
        let target = album(&conn, &profile_id, "Album");
        let other = album(&conn, &profile_id, "Other");
        let first = a_work(&conn, &profile_id, "First");
        let moved = a_work(&conn, &profile_id, "Moved");
        let fresh = a_work(&conn, &profile_id, "Fresh");
        set_contents(&conn, &target.id, std::slice::from_ref(&first)).unwrap();
        set_contents(&conn, &other.id, std::slice::from_ref(&moved)).unwrap();

        append_at(&conn, &target.id, &[fresh.clone(), moved.clone()], &now()).unwrap();

        assert_eq!(
            get(&conn, &target.id).unwrap().unwrap().work_ids,
            vec![first, fresh, moved.clone()],
            "after the last one, in the order given"
        );
        assert!(
            get(&conn, &other.id).unwrap().unwrap().work_ids.is_empty(),
            "a work is in one collection at most"
        );
        assert!(append_at(&conn, "nope", &[moved], &now()).is_err());
    }

    #[test]
    fn placements_put_back_exactly_where_works_stood() {
        let (conn, profile_id) = fixtures::workspace();
        let kept = album(&conn, &profile_id, "Album");
        let other = album(&conn, &profile_id, "Other");
        let one = a_work(&conn, &profile_id, "One");
        let two = a_work(&conn, &profile_id, "Two");
        let loose = a_work(&conn, &profile_id, "Loose");
        set_contents(&conn, &kept.id, &[one.clone(), two.clone()]).unwrap();
        let ids = [one.clone(), two.clone(), loose.clone(), "gone".to_owned()];
        let before = placements(&conn, &profile_id, &ids).unwrap();
        assert_eq!(
            before.len(),
            3,
            "a work the profile does not hold has no place"
        );

        set_contents(&conn, &other.id, &[loose.clone(), two.clone(), one.clone()]).unwrap();
        restore_at(&conn, &before, &now()).unwrap();

        assert_eq!(
            get(&conn, &kept.id).unwrap().unwrap().work_ids,
            vec![one, two]
        );
        assert!(get(&conn, &other.id).unwrap().unwrap().work_ids.is_empty());
        assert!(
            work::get(&conn, &loose)
                .unwrap()
                .unwrap()
                .collection_id
                .is_none()
        );
    }

    #[test]
    fn set_contents_on_an_unknown_collection_fails() {
        let (conn, _) = fixtures::workspace();

        assert!(set_contents(&conn, "nope", &[]).is_err());
    }

    #[test]
    fn a_collection_carries_a_goal_and_refuses_one_that_is_not() {
        let (conn, profile_id) = fixtures::workspace();
        let album = album(&conn, &profile_id, "Twelve songs");
        assert_eq!(album.target_size, None);

        let aimed = update(
            &conn,
            &album.id,
            CollectionPatch {
                target_size: Some(Some(12)),
                due_on: Some(Some("2027-03-01".into())),
                ..CollectionPatch::default()
            },
        )
        .unwrap();
        assert_eq!(aimed.target_size, Some(12));
        assert_eq!(aimed.due_on.as_deref(), Some("2027-03-01"));

        for patch in [
            CollectionPatch {
                target_size: Some(Some(0)),
                ..CollectionPatch::default()
            },
            CollectionPatch {
                due_on: Some(Some("March".into())),
                ..CollectionPatch::default()
            },
            CollectionPatch {
                due_on: Some(Some("2027-02-30".into())),
                ..CollectionPatch::default()
            },
        ] {
            assert!(update(&conn, &album.id, patch).is_err());
        }
        let cleared = update(
            &conn,
            &album.id,
            CollectionPatch {
                target_size: Some(None),
                ..CollectionPatch::default()
            },
        )
        .unwrap();
        assert_eq!(cleared.target_size, None);
        assert_eq!(cleared.due_on.as_deref(), Some("2027-03-01"));
    }
}
