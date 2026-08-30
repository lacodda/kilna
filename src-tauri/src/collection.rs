use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::error::{Error, Result};
use crate::time::now;

/// An album, a book, a season. One level deep on purpose: a collection never
/// contains a collection. Nesting buys arbitrary depth and costs every screen
/// a tree — see ADR 0001.
#[derive(Debug, Clone, Serialize)]
pub struct Collection {
    pub id: String,
    pub profile_id: String,
    pub kind: String,
    pub title: String,
    pub description: Option<String>,
    pub position: i64,
    pub meta: Map<String, Value>,
    pub created_at: String,
    pub updated_at: String,
    /// How many works sit in it.
    pub works: i64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct NewCollection {
    pub kind: String,
    pub title: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub meta: Option<Map<String, Value>>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct CollectionPatch {
    pub kind: Option<String>,
    pub title: Option<String>,
    pub description: Option<Option<String>>,
    pub meta: Option<Map<String, Value>>,
}

const SELECT_COLLECTION: &str = "SELECT c.id, c.profile_id, c.kind, c.title, c.description, c.position, c.meta, \
     c.created_at, c.updated_at, \
     (SELECT count(*) FROM work WHERE work.collection_id = c.id) AS works \
     FROM collection c";

pub fn create(conn: &Connection, profile_id: &str, new: NewCollection) -> Result<Collection> {
    let id = uuid::Uuid::new_v4().to_string();
    let timestamp = now();

    let position: i64 = conn.query_row(
        "SELECT coalesce(max(position), -1) + 1 FROM collection WHERE profile_id = ?1",
        params![profile_id],
        |row| row.get(0),
    )?;

    conn.execute(
        "INSERT INTO collection (id, profile_id, kind, title, description, position, meta, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?8)",
        params![
            id,
            profile_id,
            new.kind,
            new.title,
            new.description,
            position,
            Value::Object(new.meta.unwrap_or_default()).to_string(),
            timestamp,
        ],
    )?;

    get(conn, &id)?.ok_or_else(|| Error::Other("the collection vanished after insert".into()))
}

pub fn get(conn: &Connection, id: &str) -> Result<Option<Collection>> {
    let raw = conn
        .query_row(
            &format!("{SELECT_COLLECTION} WHERE c.id = ?1"),
            params![id],
            read_row,
        )
        .optional()?;

    raw.map(RawCollection::into_collection).transpose()
}

pub fn list(conn: &Connection, profile_id: &str) -> Result<Vec<Collection>> {
    let mut statement = conn.prepare(&format!(
        "{SELECT_COLLECTION} WHERE c.profile_id = ?1 ORDER BY c.position, c.title"
    ))?;
    let raw = statement
        .query_map(params![profile_id], read_row)?
        .collect::<rusqlite::Result<Vec<_>>>()?;

    raw.into_iter()
        .map(RawCollection::into_collection)
        .collect()
}

pub fn update(conn: &Connection, id: &str, patch: CollectionPatch) -> Result<Collection> {
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

    if assignments.is_empty() {
        return get(conn, id)?.ok_or_else(|| unknown(id));
    }

    set(&mut assignments, &mut values, "updated_at", Box::new(now()));
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
pub fn set_contents(conn: &mut Connection, id: &str, work_ids: &[String]) -> Result<()> {
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

    let tx = conn.transaction()?;
    let timestamp = now();

    tx.execute(
        "UPDATE work SET collection_id = NULL, updated_at = ?2 WHERE collection_id = ?1",
        params![id, timestamp],
    )?;

    for (position, work_id) in work_ids.iter().enumerate() {
        tx.execute(
            "UPDATE work SET collection_id = ?1, position = ?2, updated_at = ?3 WHERE id = ?4",
            params![id, position as i64, timestamp, work_id],
        )?;
    }

    tx.commit()?;
    Ok(())
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
    works: i64,
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
        works: row.get(9)?,
    })
}

impl RawCollection {
    fn into_collection(self) -> Result<Collection> {
        Ok(Collection {
            meta: serde_json::from_str(&self.meta)?,
            id: self.id,
            profile_id: self.profile_id,
            kind: self.kind,
            title: self.title,
            description: self.description,
            position: self.position,
            created_at: self.created_at,
            updated_at: self.updated_at,
            works: self.works,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;
    use crate::profile;
    use crate::work::{self, NewWork, WorkFilter};

    fn workspace() -> (Connection, String) {
        let conn = db::open_in_memory().unwrap();
        profile::seed(&conn).unwrap();
        let profile_id = profile::active(&conn).unwrap().unwrap().id;
        (conn, profile_id)
    }

    fn album(conn: &Connection, profile_id: &str, title: &str) -> Collection {
        create(
            conn,
            profile_id,
            NewCollection {
                kind: "album".into(),
                title: title.into(),
                description: None,
                meta: None,
            },
        )
        .unwrap()
    }

    fn a_work(conn: &Connection, profile_id: &str, title: &str) -> String {
        work::create(
            conn,
            profile_id,
            NewWork {
                kind: "song".into(),
                title: title.into(),
                ..NewWork::default()
            },
        )
        .unwrap()
        .id
    }

    #[test]
    fn a_new_collection_is_empty_and_positioned_last() {
        let (conn, profile_id) = workspace();

        let first = album(&conn, &profile_id, "First");
        let second = album(&conn, &profile_id, "Second");

        assert_eq!(first.works, 0);
        assert_eq!(first.position, 0);
        assert_eq!(second.position, 1);
    }

    #[test]
    fn set_contents_orders_the_works_and_counts_them() {
        let (mut conn, profile_id) = workspace();
        let collection = album(&conn, &profile_id, "Album");
        let one = a_work(&conn, &profile_id, "One");
        let two = a_work(&conn, &profile_id, "Two");

        set_contents(&mut conn, &collection.id, &[two.clone(), one.clone()]).unwrap();

        let reloaded = get(&conn, &collection.id).unwrap().unwrap();
        assert_eq!(reloaded.works, 2);

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
        let (mut conn, profile_id) = workspace();
        let collection = album(&conn, &profile_id, "Album");
        let stays = a_work(&conn, &profile_id, "Stays");
        let leaves = a_work(&conn, &profile_id, "Leaves");
        set_contents(&mut conn, &collection.id, &[stays.clone(), leaves.clone()]).unwrap();

        set_contents(&mut conn, &collection.id, &[stays]).unwrap();

        assert_eq!(get(&conn, &collection.id).unwrap().unwrap().works, 1);
        let loose = work::get(&conn, &leaves).unwrap().unwrap();
        assert!(loose.collection_id.is_none(), "the work survives, loose");
    }

    #[test]
    fn deleting_a_collection_leaves_its_works_alone() {
        let (mut conn, profile_id) = workspace();
        let collection = album(&conn, &profile_id, "Album");
        let work_id = a_work(&conn, &profile_id, "Inside");
        set_contents(&mut conn, &collection.id, std::slice::from_ref(&work_id)).unwrap();

        delete(&conn, &collection.id).unwrap();

        let survivor = work::get(&conn, &work_id).unwrap().unwrap();
        assert!(survivor.collection_id.is_none());
    }

    #[test]
    fn collections_are_scoped_to_a_profile() {
        let (conn, profile_id) = workspace();
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
    fn set_contents_on_an_unknown_collection_fails() {
        let (mut conn, _) = workspace();

        assert!(set_contents(&mut conn, "nope", &[]).is_err());
    }
}
