pub mod status;
pub mod version;

use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::error::{Error, Result};
use crate::time::now;

/// A work as the frontend sees it.
#[derive(Debug, Clone, Serialize)]
pub struct Work {
    pub id: String,
    pub profile_id: String,
    pub collection_id: Option<String>,
    pub kind: String,
    pub title: String,
    pub status: String,
    /// Set when a person chose the status by hand. While it holds a value the
    /// automation leaves the work alone — see [`status`].
    pub status_pinned_at: Option<String>,
    pub meta: Map<String, Value>,
    /// The author's own words for what this is. Free text, completed from what
    /// the workspace already holds rather than chosen from a list.
    pub tags: Vec<String>,
    /// Keys into the profile's `marks`. A key the profile no longer defines is
    /// kept in the row but not drawn — renaming a mark never touches a work.
    pub marks: Vec<String>,
    pub current_version_id: Option<String>,
    pub position: i64,
    pub created_at: String,
    pub updated_at: String,
}

/// Fields accepted when creating a work. Everything else is derived.
///
/// `Default` so a caller — a test, an importer — names only what it means and
/// is not rewritten every time the shape gains a field.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct NewWork {
    pub kind: String,
    pub title: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub marks: Vec<String>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub collection_id: Option<String>,
    #[serde(default)]
    pub meta: Option<Map<String, Value>>,
}

/// Fields that may be changed. A field left as `None` is untouched, which is
/// why every one of them is optional rather than defaulted.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct WorkPatch {
    pub title: Option<String>,
    pub status: Option<String>,
    pub kind: Option<String>,
    pub collection_id: Option<Option<String>>,
    pub meta: Option<Map<String, Value>>,
    pub tags: Option<Vec<String>>,
    pub marks: Option<Vec<String>>,
    pub current_version_id: Option<Option<String>>,
}

/// Narrowing applied to a listing.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct WorkFilter {
    pub status: Option<String>,
    pub kind: Option<String>,
    pub collection_id: Option<String>,
    /// Case-insensitive substring of the title.
    pub search: Option<String>,
}

const SELECT_WORK: &str = "SELECT id, profile_id, collection_id, kind, title, status, \
     status_pinned_at, meta, tags, marks, current_version_id, position, created_at, updated_at \
     FROM work";

/// Create a work in the given profile.
///
/// The status defaults to the profile's first one, so a caller does not have to
/// know the vocabulary to add something.
pub fn create(conn: &Connection, profile_id: &str, new: NewWork) -> Result<Work> {
    let status = match new.status {
        Some(status) => status,
        None => default_status(conn, profile_id)?,
    };

    let timestamp = now();
    let id = uuid::Uuid::new_v4().to_string();
    let meta = Value::Object(new.meta.unwrap_or_default()).to_string();
    let tags = serde_json::to_string(&new.tags)?;
    let marks = serde_json::to_string(&new.marks)?;

    // New works go to the end of the profile's list.
    let position: i64 = conn.query_row(
        "SELECT coalesce(max(position), -1) + 1 FROM work WHERE profile_id = ?1",
        params![profile_id],
        |row| row.get(0),
    )?;

    conn.execute(
        "INSERT INTO work (id, profile_id, collection_id, kind, title, status, meta, tags, marks, position, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?11)",
        params![
            id,
            profile_id,
            new.collection_id,
            new.kind,
            new.title,
            status,
            meta,
            tags,
            marks,
            position,
            timestamp,
        ],
    )?;

    get(conn, &id)?.ok_or_else(|| Error::Other("the work vanished after insert".into()))
}

/// The status a work starts in: the profile's word for a draft.
///
/// Nothing has happened to a work at the moment it is created, and "nothing has
/// happened yet" is exactly what [`Derive::Draft`] means — so the automation
/// would derive this same value on its first pass. Falls back to the first
/// status in the list for a profile that names no draft, which is the older
/// behaviour and still the only sensible guess.
fn default_status(conn: &Connection, profile_id: &str) -> Result<String> {
    let config = crate::profile::config_for(conn, profile_id)?;

    config
        .statuses
        .iter()
        .find(|status| status.derive == crate::profile::config::Derive::Draft)
        .or_else(|| config.statuses.first())
        .map(|status| status.key.clone())
        .ok_or_else(|| Error::Other("the profile defines no statuses".into()))
}

pub fn get(conn: &Connection, id: &str) -> Result<Option<Work>> {
    let raw = conn
        .query_row(
            &format!("{SELECT_WORK} WHERE id = ?1"),
            params![id],
            read_row,
        )
        .optional()?;

    raw.map(RawWork::into_work).transpose()
}

/// Works in a profile, newest first, narrowed by `filter`.
pub fn list(conn: &Connection, profile_id: &str, filter: &WorkFilter) -> Result<Vec<Work>> {
    // Each clause is optional, so the SQL is assembled rather than fixed; the
    // values stay bound, never interpolated.
    let mut sql = format!("{SELECT_WORK} WHERE profile_id = ?1");
    let mut values: Vec<Box<dyn rusqlite::ToSql>> = vec![Box::new(profile_id.to_owned())];

    if let Some(status) = &filter.status {
        values.push(Box::new(status.clone()));
        sql.push_str(&format!(" AND status = ?{}", values.len()));
    }
    if let Some(kind) = &filter.kind {
        values.push(Box::new(kind.clone()));
        sql.push_str(&format!(" AND kind = ?{}", values.len()));
    }
    if let Some(collection_id) = &filter.collection_id {
        values.push(Box::new(collection_id.clone()));
        sql.push_str(&format!(" AND collection_id = ?{}", values.len()));
    }
    // The search is deliberately not a LIKE here: SQLite ignores case for ASCII
    // only, so filtering "Гавань" by "гавань" found nothing at all. Titles are
    // matched after the query instead, the same way the palette does it.

    sql.push_str(" ORDER BY updated_at DESC, title, rowid DESC");

    let mut statement = conn.prepare(&sql)?;
    let params = rusqlite::params_from_iter(values.iter().map(AsRef::as_ref));
    let raw = statement
        .query_map(params, read_row)?
        .collect::<rusqlite::Result<Vec<_>>>()?;

    let works: Vec<Work> = raw
        .into_iter()
        .map(RawWork::into_work)
        .collect::<Result<Vec<_>>>()?;

    let Some(search) = &filter.search else {
        return Ok(works);
    };
    let needle = search.trim().to_lowercase();
    if needle.is_empty() {
        return Ok(works);
    }

    Ok(works
        .into_iter()
        .filter(|work| crate::search::matches(&work.title, &needle))
        .collect())
}

/// Apply a patch. Returns the updated work.
pub fn update(conn: &Connection, id: &str, patch: WorkPatch) -> Result<Work> {
    let mut assignments: Vec<String> = Vec::new();
    let mut values: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

    // Each present field contributes one assignment and one bound value; the
    // placeholder number is the value's position, so the two stay in step.
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
    // Choosing a status by hand is what pins it: there is no separate "pin"
    // gesture to forget. Passing the value the work already holds still pins,
    // and deliberately so — re-picking the current status is how a person says
    // "this one, keep it" when the automation disagrees.
    if let Some(status) = patch.status {
        set(&mut assignments, &mut values, "status", Box::new(status));
        set(
            &mut assignments,
            &mut values,
            "status_pinned_at",
            Box::new(now()),
        );
    }
    if let Some(kind) = patch.kind {
        set(&mut assignments, &mut values, "kind", Box::new(kind));
    }
    if let Some(collection_id) = patch.collection_id {
        set(
            &mut assignments,
            &mut values,
            "collection_id",
            Box::new(collection_id),
        );
    }
    if let Some(tags) = patch.tags {
        // Trimmed, emptied entries dropped, and deduplicated case-insensitively
        // — "Winter" typed beside an existing "winter" is the same tag, and two
        // spellings of one word split a catalogue in half without saying so.
        let mut kept: Vec<String> = Vec::new();
        for tag in tags {
            let tag = tag.trim().to_owned();
            if tag.is_empty() {
                continue;
            }
            if !kept
                .iter()
                .any(|existing| crate::search::fold(existing) == crate::search::fold(&tag))
            {
                kept.push(tag);
            }
        }
        set(
            &mut assignments,
            &mut values,
            "tags",
            Box::new(serde_json::to_string(&kept)?),
        );
    }
    if let Some(marks) = patch.marks {
        set(
            &mut assignments,
            &mut values,
            "marks",
            Box::new(serde_json::to_string(&marks)?),
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
    if let Some(version_id) = patch.current_version_id {
        set(
            &mut assignments,
            &mut values,
            "current_version_id",
            Box::new(version_id),
        );
    }

    if assignments.is_empty() {
        return get(conn, id)?.ok_or_else(|| unknown_work(id));
    }

    set(&mut assignments, &mut values, "updated_at", Box::new(now()));
    values.push(Box::new(id.to_owned()));

    let sql = format!(
        "UPDATE work SET {} WHERE id = ?{}",
        assignments.join(", "),
        values.len()
    );
    let params = rusqlite::params_from_iter(values.iter().map(AsRef::as_ref));
    let changed = conn.execute(&sql, params)?;

    if changed == 0 {
        return Err(unknown_work(id));
    }

    get(conn, id)?.ok_or_else(|| unknown_work(id))
}

/// Delete a work along with everything that hangs off it.
/// Every tag in use in a profile, most used first.
///
/// This is what makes a free-text field converge on a vocabulary instead of
/// scattering: the box offers what the workspace already says, so the second
/// winter song is tagged from the list rather than retyped. Counted rather than
/// merely listed, because the tags worth offering first are the ones already
/// carrying weight.
pub fn tags(conn: &Connection, profile_id: &str) -> Result<Vec<(String, i64)>> {
    let mut statement = conn.prepare(
        "SELECT json_each.value AS tag, count(*) AS uses
         FROM work, json_each(work.tags)
         WHERE work.profile_id = ?1
         GROUP BY tag
         ORDER BY uses DESC, tag",
    )?;

    let rows = statement.query_map(params![profile_id], |row| Ok((row.get(0)?, row.get(1)?)))?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

pub fn delete(conn: &Connection, id: &str) -> Result<()> {
    // Versions, scores, releases, notes and assets go with it via ON DELETE CASCADE.
    let changed = conn.execute("DELETE FROM work WHERE id = ?1", params![id])?;

    if changed == 0 {
        return Err(unknown_work(id));
    }
    Ok(())
}

fn unknown_work(id: &str) -> Error {
    Error::not_found("work", id)
}

/// A row before its JSON columns are parsed.
struct RawWork {
    id: String,
    profile_id: String,
    collection_id: Option<String>,
    kind: String,
    title: String,
    status: String,
    status_pinned_at: Option<String>,
    meta: String,
    tags: String,
    marks: String,
    current_version_id: Option<String>,
    position: i64,
    created_at: String,
    updated_at: String,
}

fn read_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<RawWork> {
    Ok(RawWork {
        id: row.get(0)?,
        profile_id: row.get(1)?,
        collection_id: row.get(2)?,
        kind: row.get(3)?,
        title: row.get(4)?,
        status: row.get(5)?,
        status_pinned_at: row.get(6)?,
        meta: row.get(7)?,
        tags: row.get(8)?,
        marks: row.get(9)?,
        current_version_id: row.get(10)?,
        position: row.get(11)?,
        created_at: row.get(12)?,
        updated_at: row.get(13)?,
    })
}

impl RawWork {
    fn into_work(self) -> Result<Work> {
        Ok(Work {
            meta: serde_json::from_str(&self.meta)?,
            tags: serde_json::from_str(&self.tags)?,
            marks: serde_json::from_str(&self.marks)?,
            id: self.id,
            profile_id: self.profile_id,
            collection_id: self.collection_id,
            kind: self.kind,
            title: self.title,
            status: self.status,
            status_pinned_at: self.status_pinned_at,
            current_version_id: self.current_version_id,
            position: self.position,
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
    use serde_json::json;

    /// A workspace with the built-in profiles installed and one active.
    fn workspace() -> (Connection, String) {
        let conn = db::open_in_memory().unwrap();
        profile::seed(&conn).unwrap();
        let profile_id = profile::active(&conn).unwrap().unwrap().id;
        (conn, profile_id)
    }

    fn song(title: &str) -> NewWork {
        NewWork {
            kind: "song".into(),
            title: title.into(),
            ..NewWork::default()
        }
    }

    #[test]
    fn tags_are_trimmed_emptied_and_deduplicated_in_any_language() {
        let (conn, profile_id) = workspace();
        let work = create(&conn, &profile_id, song("Winter road")).unwrap();

        let updated = update(
            &conn,
            &work.id,
            WorkPatch {
                tags: Some(vec![
                    "  winter ".into(),
                    "Winter".into(),
                    "".into(),
                    "   ".into(),
                    "зима".into(),
                    "ЗИМА".into(),
                ]),
                ..WorkPatch::default()
            },
        )
        .unwrap();

        // One "winter" and one "зима": a second spelling of the same word would
        // split the catalogue in half without ever saying so, and case folding
        // has to know Cyrillic for that to hold.
        assert_eq!(updated.tags, vec!["winter".to_owned(), "зима".to_owned()]);
    }

    #[test]
    fn the_first_spelling_of_a_tag_is_the_one_kept() {
        let (conn, profile_id) = workspace();
        let work = create(&conn, &profile_id, song("Winter road")).unwrap();

        let updated = update(
            &conn,
            &work.id,
            WorkPatch {
                tags: Some(vec!["Winter".into(), "winter".into()]),
                ..WorkPatch::default()
            },
        )
        .unwrap();

        // Not lower-cased on the way in: the author's capitalisation is theirs,
        // and folding is for comparing, not for storing.
        assert_eq!(updated.tags, vec!["Winter".to_owned()]);
    }

    #[test]
    fn tags_and_marks_are_separate_lists() {
        let (conn, profile_id) = workspace();
        let work = create(&conn, &profile_id, song("Winter road")).unwrap();

        let tagged = update(
            &conn,
            &work.id,
            WorkPatch {
                tags: Some(vec!["winter".into()]),
                marks: Some(vec!["on-fire".into()]),
                ..WorkPatch::default()
            },
        )
        .unwrap();
        assert_eq!(tagged.tags, vec!["winter".to_owned()]);
        assert_eq!(tagged.marks, vec!["on-fire".to_owned()]);

        // Taking the flag off leaves the vocabulary alone: a mark is about this
        // week, a tag is what the work is.
        let cleared = update(
            &conn,
            &work.id,
            WorkPatch {
                marks: Some(Vec::new()),
                ..WorkPatch::default()
            },
        )
        .unwrap();
        assert_eq!(cleared.tags, vec!["winter".to_owned()]);
        assert!(cleared.marks.is_empty());
    }

    #[test]
    fn a_patch_that_names_neither_leaves_both_alone() {
        let (conn, profile_id) = workspace();
        let work = create(&conn, &profile_id, song("Winter road")).unwrap();
        update(
            &conn,
            &work.id,
            WorkPatch {
                tags: Some(vec!["winter".into()]),
                marks: Some(vec!["on-fire".into()]),
                ..WorkPatch::default()
            },
        )
        .unwrap();

        let renamed = update(
            &conn,
            &work.id,
            WorkPatch {
                title: Some("Winter road, again".into()),
                ..WorkPatch::default()
            },
        )
        .unwrap();

        assert_eq!(renamed.tags, vec!["winter".to_owned()]);
        assert_eq!(renamed.marks, vec!["on-fire".to_owned()]);
    }

    #[test]
    fn tags_are_counted_across_the_profile_most_used_first() {
        let (conn, profile_id) = workspace();
        for (title, tags) in [
            ("One", vec!["winter", "quiet"]),
            ("Two", vec!["winter"]),
            ("Three", vec!["winter", "loud"]),
        ] {
            let work = create(&conn, &profile_id, song(title)).unwrap();
            update(
                &conn,
                &work.id,
                WorkPatch {
                    tags: Some(tags.into_iter().map(str::to_owned).collect()),
                    ..WorkPatch::default()
                },
            )
            .unwrap();
        }

        let counted = tags(&conn, &profile_id).unwrap();
        assert_eq!(counted.first(), Some(&("winter".to_owned(), 3)));
        // The rest sort by name once their counts tie, so the list is stable
        // rather than reordering itself between reads.
        let names: Vec<&str> = counted.iter().map(|(tag, _)| tag.as_str()).collect();
        assert_eq!(names, vec!["winter", "loud", "quiet"]);
    }

    #[test]
    fn create_falls_back_to_the_profiles_first_status() {
        let (conn, profile_id) = workspace();

        let work = create(&conn, &profile_id, song("First light")).unwrap();

        assert_eq!(work.status, "draft");
        assert_eq!(work.title, "First light");
        assert!(work.current_version_id.is_none());
    }

    #[test]
    fn create_keeps_meta_as_a_json_object() {
        let (conn, profile_id) = workspace();
        let mut new = song("Tempo test");
        new.meta = json!({ "bpm": 128, "key": "Am" }).as_object().cloned();

        let work = create(&conn, &profile_id, new).unwrap();

        assert_eq!(work.meta.get("bpm").unwrap(), &json!(128));
        // Round-trips through the database rather than only through the struct.
        let reloaded = get(&conn, &work.id).unwrap().unwrap();
        assert_eq!(reloaded.meta.get("key").unwrap(), &json!("Am"));
    }

    #[test]
    fn works_are_positioned_in_order_of_creation() {
        let (conn, profile_id) = workspace();

        let first = create(&conn, &profile_id, song("One")).unwrap();
        let second = create(&conn, &profile_id, song("Two")).unwrap();

        assert_eq!(first.position, 0);
        assert_eq!(second.position, 1);
    }

    #[test]
    fn list_filters_by_status_kind_and_title() {
        let (conn, profile_id) = workspace();
        create(&conn, &profile_id, song("Winter road")).unwrap();
        let mut instrumental = song("Winter theme");
        instrumental.kind = "instrumental".into();
        create(&conn, &profile_id, instrumental).unwrap();
        let mut released = song("Old release");
        released.status = Some("released".into());
        create(&conn, &profile_id, released).unwrap();

        let all = list(&conn, &profile_id, &WorkFilter::default()).unwrap();
        assert_eq!(all.len(), 3);

        let songs = list(
            &conn,
            &profile_id,
            &WorkFilter {
                kind: Some("song".into()),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(songs.len(), 2);

        let drafts = list(
            &conn,
            &profile_id,
            &WorkFilter {
                status: Some("draft".into()),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(drafts.len(), 2);

        let winter = list(
            &conn,
            &profile_id,
            &WorkFilter {
                search: Some("winter".into()),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(winter.len(), 2, "search is case-insensitive");
    }

    // SQLite's own LIKE would have failed this: it ignores case for ASCII and
    // nothing else, so a Russian title was unfindable unless typed exactly.
    #[test]
    fn search_ignores_case_in_russian_too() {
        let (conn, profile_id) = workspace();
        create(&conn, &profile_id, song("Гавань огней")).unwrap();

        for query in ["гавань", "ГАВАНЬ", "огней"] {
            let found = list(
                &conn,
                &profile_id,
                &WorkFilter {
                    search: Some(query.into()),
                    ..Default::default()
                },
            )
            .unwrap();
            assert_eq!(found.len(), 1, "`{query}` should have found the work");
        }
    }

    #[test]
    fn list_is_scoped_to_one_profile() {
        let (conn, profile_id) = workspace();
        create(&conn, &profile_id, song("Mine")).unwrap();
        conn.execute(
            "INSERT INTO profile (id, key, name, config, is_active, is_builtin, created_at, updated_at)
             SELECT 'other', 'other', 'Other', config, 0, 0, created_at, updated_at FROM profile LIMIT 1",
            [],
        )
        .unwrap();
        create(&conn, "other", song("Theirs")).unwrap();

        let mine = list(&conn, &profile_id, &WorkFilter::default()).unwrap();

        assert_eq!(mine.len(), 1);
        assert_eq!(mine[0].title, "Mine");
    }

    #[test]
    fn update_touches_only_the_given_fields() {
        let (conn, profile_id) = workspace();
        let work = create(&conn, &profile_id, song("Working title")).unwrap();

        let updated = update(
            &conn,
            &work.id,
            WorkPatch {
                title: Some("Final title".into()),
                ..Default::default()
            },
        )
        .unwrap();

        assert_eq!(updated.title, "Final title");
        assert_eq!(updated.status, work.status, "status was not in the patch");
        assert_eq!(updated.kind, work.kind);
    }

    #[test]
    fn update_can_clear_a_nullable_field() {
        let (conn, profile_id) = workspace();
        let mut new = song("Grouped");
        conn.execute(
            "INSERT INTO collection (id, profile_id, kind, title, created_at, updated_at)
             VALUES ('c1', ?1, 'album', 'Album', '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')",
            params![profile_id],
        )
        .unwrap();
        new.collection_id = Some("c1".into());
        let work = create(&conn, &profile_id, new).unwrap();
        assert_eq!(work.collection_id.as_deref(), Some("c1"));

        let updated = update(
            &conn,
            &work.id,
            WorkPatch {
                collection_id: Some(None),
                ..Default::default()
            },
        )
        .unwrap();

        assert!(updated.collection_id.is_none());
    }

    #[test]
    fn update_of_an_unknown_work_fails() {
        let (conn, _) = workspace();

        let result = update(
            &conn,
            "nope",
            WorkPatch {
                title: Some("x".into()),
                ..Default::default()
            },
        );

        assert!(result.is_err());
    }

    #[test]
    fn deleting_a_work_takes_its_notes_with_it() {
        let (conn, profile_id) = workspace();
        let work = create(&conn, &profile_id, song("Doomed")).unwrap();
        conn.execute(
            "INSERT INTO note (id, profile_id, work_id, body, created_at, updated_at)
             VALUES ('n1', ?1, ?2, 'a thought', '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')",
            params![profile_id, work.id],
        )
        .unwrap();

        delete(&conn, &work.id).unwrap();

        let notes: i64 = conn
            .query_row("SELECT count(*) FROM note", [], |row| row.get(0))
            .unwrap();
        assert_eq!(notes, 0);
        assert!(get(&conn, &work.id).unwrap().is_none());
    }
}
