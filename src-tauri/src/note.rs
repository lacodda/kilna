use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use crate::time::now;

/// Ideas, lore, reference — one type distinguished by `kind` and tags rather
/// than by four separate subsystems. The predecessor built those subsystems and
/// they went unused; see the vision notes.
#[derive(Debug, Clone, Serialize)]
pub struct Note {
    pub id: String,
    pub profile_id: String,
    pub work_id: Option<String>,
    pub kind: String,
    pub title: Option<String>,
    pub body: String,
    pub tags: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct NewNote {
    pub body: String,
    #[serde(default)]
    pub kind: Option<String>,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub work_id: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct NotePatch {
    pub title: Option<Option<String>>,
    pub body: Option<String>,
    pub kind: Option<String>,
    pub tags: Option<Vec<String>>,
    pub work_id: Option<Option<String>>,
}

/// Narrowing applied to a listing.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct NoteFilter {
    pub work_id: Option<String>,
    pub kind: Option<String>,
    /// Notes carrying this tag.
    pub tag: Option<String>,
    /// Case-insensitive substring of the title or body.
    pub search: Option<String>,
}

const SELECT_NOTE: &str =
    "SELECT id, profile_id, work_id, kind, title, body, tags, created_at, updated_at FROM note";

pub fn create(conn: &Connection, profile_id: &str, new: NewNote) -> Result<Note> {
    let id = uuid::Uuid::new_v4().to_string();
    let timestamp = now();

    conn.execute(
        "INSERT INTO note (id, profile_id, work_id, kind, title, body, tags, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?8)",
        params![
            id,
            profile_id,
            new.work_id,
            new.kind.unwrap_or_else(|| "note".into()),
            new.title,
            new.body,
            serde_json::to_string(&new.tags)?,
            timestamp,
        ],
    )?;

    get(conn, &id)?.ok_or_else(|| Error::Other("the note vanished after insert".into()))
}

pub fn get(conn: &Connection, id: &str) -> Result<Option<Note>> {
    let raw = conn
        .query_row(
            &format!("{SELECT_NOTE} WHERE id = ?1"),
            params![id],
            read_row,
        )
        .optional()?;

    raw.map(RawNote::into_note).transpose()
}

/// Notes in a profile, most recently touched first.
pub fn list(conn: &Connection, profile_id: &str, filter: &NoteFilter) -> Result<Vec<Note>> {
    let mut sql = format!("{SELECT_NOTE} WHERE profile_id = ?1");
    let mut values: Vec<Box<dyn rusqlite::ToSql>> = vec![Box::new(profile_id.to_owned())];

    if let Some(work_id) = &filter.work_id {
        values.push(Box::new(work_id.clone()));
        sql.push_str(&format!(" AND work_id = ?{}", values.len()));
    }
    if let Some(kind) = &filter.kind {
        values.push(Box::new(kind.clone()));
        sql.push_str(&format!(" AND kind = ?{}", values.len()));
    }
    if let Some(tag) = &filter.tag {
        // Tags are a JSON array; matching an element beats a LIKE over the
        // serialised text, which would also match a tag that merely contains it.
        values.push(Box::new(tag.clone()));
        sql.push_str(&format!(
            " AND EXISTS (SELECT 1 FROM json_each(note.tags) WHERE json_each.value = ?{})",
            values.len()
        ));
    }
    if let Some(search) = &filter.search {
        values.push(Box::new(format!("%{search}%")));
        let n = values.len();
        sql.push_str(&format!(
            " AND (coalesce(title, '') LIKE ?{n} OR body LIKE ?{n})"
        ));
    }

    sql.push_str(" ORDER BY updated_at DESC");

    let mut statement = conn.prepare(&sql)?;
    let params = rusqlite::params_from_iter(values.iter().map(AsRef::as_ref));
    let raw = statement
        .query_map(params, read_row)?
        .collect::<rusqlite::Result<Vec<_>>>()?;

    raw.into_iter().map(RawNote::into_note).collect()
}

pub fn update(conn: &Connection, id: &str, patch: NotePatch) -> Result<Note> {
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

    if let Some(title) = patch.title {
        set(&mut assignments, &mut values, "title", Box::new(title));
    }
    if let Some(body) = patch.body {
        set(&mut assignments, &mut values, "body", Box::new(body));
    }
    if let Some(kind) = patch.kind {
        set(&mut assignments, &mut values, "kind", Box::new(kind));
    }
    if let Some(tags) = patch.tags {
        set(
            &mut assignments,
            &mut values,
            "tags",
            Box::new(serde_json::to_string(&tags)?),
        );
    }
    if let Some(work_id) = patch.work_id {
        set(&mut assignments, &mut values, "work_id", Box::new(work_id));
    }

    if assignments.is_empty() {
        return get(conn, id)?.ok_or_else(|| unknown_note(id));
    }

    set(&mut assignments, &mut values, "updated_at", Box::new(now()));
    values.push(Box::new(id.to_owned()));

    let sql = format!(
        "UPDATE note SET {} WHERE id = ?{}",
        assignments.join(", "),
        values.len()
    );
    let params = rusqlite::params_from_iter(values.iter().map(AsRef::as_ref));

    if conn.execute(&sql, params)? == 0 {
        return Err(unknown_note(id));
    }

    get(conn, id)?.ok_or_else(|| unknown_note(id))
}

pub fn delete(conn: &Connection, id: &str) -> Result<()> {
    if conn.execute("DELETE FROM note WHERE id = ?1", params![id])? == 0 {
        return Err(unknown_note(id));
    }
    Ok(())
}

/// Every tag in use in a profile, with how often it appears.
pub fn tags(conn: &Connection, profile_id: &str) -> Result<Vec<(String, i64)>> {
    let mut statement = conn.prepare(
        "SELECT json_each.value AS tag, count(*) AS uses
         FROM note, json_each(note.tags)
         WHERE note.profile_id = ?1
         GROUP BY tag
         ORDER BY uses DESC, tag",
    )?;

    let rows = statement.query_map(params![profile_id], |row| Ok((row.get(0)?, row.get(1)?)))?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

fn unknown_note(id: &str) -> Error {
    Error::not_found("note", id)
}

struct RawNote {
    id: String,
    profile_id: String,
    work_id: Option<String>,
    kind: String,
    title: Option<String>,
    body: String,
    tags: String,
    created_at: String,
    updated_at: String,
}

fn read_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<RawNote> {
    Ok(RawNote {
        id: row.get(0)?,
        profile_id: row.get(1)?,
        work_id: row.get(2)?,
        kind: row.get(3)?,
        title: row.get(4)?,
        body: row.get(5)?,
        tags: row.get(6)?,
        created_at: row.get(7)?,
        updated_at: row.get(8)?,
    })
}

impl RawNote {
    fn into_note(self) -> Result<Note> {
        Ok(Note {
            tags: serde_json::from_str(&self.tags)?,
            id: self.id,
            profile_id: self.profile_id,
            work_id: self.work_id,
            kind: self.kind,
            title: self.title,
            body: self.body,
            created_at: self.created_at,
            updated_at: self.updated_at,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;
    use crate::profile;
    use crate::work::{self, NewWork};

    fn workspace() -> (Connection, String) {
        let conn = db::open_in_memory().unwrap();
        profile::seed(&conn).unwrap();
        let profile_id = profile::active(&conn).unwrap().unwrap().id;
        (conn, profile_id)
    }

    fn note(body: &str, tags: &[&str]) -> NewNote {
        NewNote {
            body: body.into(),
            kind: None,
            title: None,
            work_id: None,
            tags: tags.iter().map(|t| (*t).to_owned()).collect(),
        }
    }

    #[test]
    fn a_note_defaults_to_the_plain_kind() {
        let (conn, profile_id) = workspace();

        let note = create(&conn, &profile_id, note("a thought", &[])).unwrap();

        assert_eq!(note.kind, "note");
        assert!(note.tags.is_empty());
    }

    #[test]
    fn tags_round_trip_through_the_database() {
        let (conn, profile_id) = workspace();

        let created = create(&conn, &profile_id, note("tagged", &["idea", "winter"])).unwrap();
        let reloaded = get(&conn, &created.id).unwrap().unwrap();

        assert_eq!(reloaded.tags, vec!["idea", "winter"]);
    }

    #[test]
    fn filtering_by_tag_matches_whole_tags_only() {
        let (conn, profile_id) = workspace();
        create(&conn, &profile_id, note("exact", &["win"])).unwrap();
        create(&conn, &profile_id, note("longer", &["winter"])).unwrap();

        let found = list(
            &conn,
            &profile_id,
            &NoteFilter {
                tag: Some("win".into()),
                ..Default::default()
            },
        )
        .unwrap();

        assert_eq!(found.len(), 1, "`winter` must not match the tag `win`");
        assert_eq!(found[0].body, "exact");
    }

    #[test]
    fn search_covers_the_title_and_the_body() {
        let (conn, profile_id) = workspace();
        let mut titled = note("unrelated body", &[]);
        titled.title = Some("Winter sketch".into());
        create(&conn, &profile_id, titled).unwrap();
        create(&conn, &profile_id, note("something about winter", &[])).unwrap();
        create(&conn, &profile_id, note("nothing relevant", &[])).unwrap();

        let found = list(
            &conn,
            &profile_id,
            &NoteFilter {
                search: Some("winter".into()),
                ..Default::default()
            },
        )
        .unwrap();

        assert_eq!(found.len(), 2);
    }

    #[test]
    fn a_note_can_be_attached_to_a_work_and_filtered_by_it() {
        let (conn, profile_id) = workspace();
        let work = work::create(
            &conn,
            &profile_id,
            NewWork {
                kind: "song".into(),
                title: "Subject".into(),
                ..NewWork::default()
            },
        )
        .unwrap();
        let mut attached = note("about this song", &[]);
        attached.work_id = Some(work.id.clone());
        create(&conn, &profile_id, attached).unwrap();
        create(&conn, &profile_id, note("loose", &[])).unwrap();

        let found = list(
            &conn,
            &profile_id,
            &NoteFilter {
                work_id: Some(work.id.clone()),
                ..Default::default()
            },
        )
        .unwrap();

        assert_eq!(found.len(), 1);
        assert_eq!(found[0].work_id.as_deref(), Some(work.id.as_str()));
    }

    #[test]
    fn update_replaces_the_whole_tag_set() {
        let (conn, profile_id) = workspace();
        let created = create(&conn, &profile_id, note("body", &["old"])).unwrap();

        let updated = update(
            &conn,
            &created.id,
            NotePatch {
                tags: Some(vec!["new".into(), "fresh".into()]),
                ..Default::default()
            },
        )
        .unwrap();

        assert_eq!(updated.tags, vec!["new", "fresh"]);
    }

    #[test]
    fn update_can_clear_the_title() {
        let (conn, profile_id) = workspace();
        let mut titled = note("body", &[]);
        titled.title = Some("Working title".into());
        let created = create(&conn, &profile_id, titled).unwrap();

        let updated = update(
            &conn,
            &created.id,
            NotePatch {
                title: Some(None),
                ..Default::default()
            },
        )
        .unwrap();

        assert!(updated.title.is_none());
    }

    #[test]
    fn tags_are_counted_across_the_profile() {
        let (conn, profile_id) = workspace();
        create(&conn, &profile_id, note("one", &["idea", "winter"])).unwrap();
        create(&conn, &profile_id, note("two", &["idea"])).unwrap();

        let tags = tags(&conn, &profile_id).unwrap();

        assert_eq!(tags[0], ("idea".to_owned(), 2));
        assert_eq!(tags[1], ("winter".to_owned(), 1));
    }

    #[test]
    fn deleting_an_unknown_note_fails() {
        let (conn, _) = workspace();

        assert!(delete(&conn, "nope").is_err());
    }
}
