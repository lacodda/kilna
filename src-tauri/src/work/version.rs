use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::error::{Error, Result};
use crate::time::now;

/// A draft kept whole. Bodies are never stored as diffs — see ADR 0002.
#[derive(Debug, Clone, Serialize)]
pub struct Version {
    pub id: String,
    pub work_id: String,
    pub role: String,
    pub revision: i64,
    pub label: Option<String>,
    pub body: String,
    pub meta: Map<String, Value>,
    pub created_at: String,
}

/// A version without its body — enough to draw a history list.
#[derive(Debug, Clone, Serialize)]
pub struct VersionSummary {
    pub id: String,
    pub work_id: String,
    pub role: String,
    pub revision: i64,
    pub label: Option<String>,
    /// Characters in the body; the list shows growth without loading it.
    pub length: i64,
    pub created_at: String,
    pub is_current: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct NewVersion {
    pub role: String,
    pub body: String,
    #[serde(default)]
    pub label: Option<String>,
    #[serde(default)]
    pub meta: Option<Map<String, Value>>,
    /// Make this the work's current version. Defaults to true: a new draft is
    /// almost always the one being worked on.
    #[serde(default = "default_true")]
    pub make_current: bool,
}

fn default_true() -> bool {
    true
}

const SELECT_VERSION: &str =
    "SELECT id, work_id, role, revision, label, body, meta, created_at FROM work_version";

/// Add a version to a work.
///
/// The revision counts up per (work, role), so lyrics and style advance
/// independently — revising a style prompt does not renumber the lyrics.
pub fn create(conn: &mut Connection, work_id: &str, new: NewVersion) -> Result<Version> {
    let tx = conn.transaction()?;

    let exists: bool = tx
        .query_row("SELECT 1 FROM work WHERE id = ?1", params![work_id], |_| {
            Ok(true)
        })
        .optional()?
        .unwrap_or(false);
    if !exists {
        return Err(Error::not_found("work", work_id));
    }

    let revision: i64 = tx.query_row(
        "SELECT coalesce(max(revision), 0) + 1 FROM work_version WHERE work_id = ?1 AND role = ?2",
        params![work_id, new.role],
        |row| row.get(0),
    )?;

    let id = uuid::Uuid::new_v4().to_string();
    let timestamp = now();

    tx.execute(
        "INSERT INTO work_version (id, work_id, role, revision, label, body, meta, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            id,
            work_id,
            new.role,
            revision,
            new.label,
            new.body,
            Value::Object(new.meta.unwrap_or_default()).to_string(),
            timestamp,
        ],
    )?;

    if new.make_current {
        tx.execute(
            "UPDATE work SET current_version_id = ?2, updated_at = ?3 WHERE id = ?1",
            params![work_id, id, timestamp],
        )?;
    } else {
        // The work still changed, even if its current version did not.
        tx.execute(
            "UPDATE work SET updated_at = ?2 WHERE id = ?1",
            params![work_id, timestamp],
        )?;
    }

    tx.commit()?;

    get(conn, &id)?.ok_or_else(|| Error::Other("the version vanished after insert".into()))
}

pub fn get(conn: &Connection, id: &str) -> Result<Option<Version>> {
    let raw = conn
        .query_row(
            &format!("{SELECT_VERSION} WHERE id = ?1"),
            params![id],
            read_row,
        )
        .optional()?;

    raw.map(RawVersion::into_version).transpose()
}

/// Every version of a work, newest first within each role.
pub fn list(conn: &Connection, work_id: &str) -> Result<Vec<VersionSummary>> {
    let mut statement = conn.prepare(
        "SELECT v.id, v.work_id, v.role, v.revision, v.label, length(v.body), v.created_at,
                v.id = coalesce(w.current_version_id, '') AS is_current
         FROM work_version v
         JOIN work w ON w.id = v.work_id
         WHERE v.work_id = ?1
         ORDER BY v.role, v.revision DESC",
    )?;

    let rows = statement.query_map(params![work_id], |row| {
        Ok(VersionSummary {
            id: row.get(0)?,
            work_id: row.get(1)?,
            role: row.get(2)?,
            revision: row.get(3)?,
            label: row.get(4)?,
            length: row.get(5)?,
            created_at: row.get(6)?,
            is_current: row.get::<_, i64>(7)? == 1,
        })
    })?;

    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

/// The newest version of a work in a given role.
pub fn latest(conn: &Connection, work_id: &str, role: &str) -> Result<Option<Version>> {
    let raw = conn
        .query_row(
            &format!(
                "{SELECT_VERSION} WHERE work_id = ?1 AND role = ?2 ORDER BY revision DESC LIMIT 1"
            ),
            params![work_id, role],
            read_row,
        )
        .optional()?;

    raw.map(RawVersion::into_version).transpose()
}

/// Point a work at one of its versions.
pub fn set_current(conn: &Connection, work_id: &str, version_id: &str) -> Result<()> {
    let belongs: bool = conn
        .query_row(
            "SELECT 1 FROM work_version WHERE id = ?1 AND work_id = ?2",
            params![version_id, work_id],
            |_| Ok(true),
        )
        .optional()?
        .unwrap_or(false);

    if !belongs {
        return Err(Error::Other(format!(
            "version `{version_id}` does not belong to work `{work_id}`"
        )));
    }

    conn.execute(
        "UPDATE work SET current_version_id = ?2, updated_at = ?3 WHERE id = ?1",
        params![work_id, version_id, now()],
    )?;
    Ok(())
}

/// Delete a version.
///
/// Deleting the current one leaves the work pointing at the newest remaining
/// version in the same role, rather than at nothing.
pub fn delete(conn: &mut Connection, id: &str) -> Result<()> {
    let tx = conn.transaction()?;

    let Some((work_id, role)) = tx
        .query_row(
            "SELECT work_id, role FROM work_version WHERE id = ?1",
            params![id],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
        )
        .optional()?
    else {
        return Err(Error::not_found("version", id));
    };

    let was_current: bool = tx
        .query_row(
            "SELECT 1 FROM work WHERE id = ?1 AND current_version_id = ?2",
            params![work_id, id],
            |_| Ok(true),
        )
        .optional()?
        .unwrap_or(false);

    tx.execute("DELETE FROM work_version WHERE id = ?1", params![id])?;

    if was_current {
        let replacement: Option<String> = tx
            .query_row(
                "SELECT id FROM work_version WHERE work_id = ?1 AND role = ?2
                 ORDER BY revision DESC LIMIT 1",
                params![work_id, role],
                |row| row.get(0),
            )
            .optional()?;

        tx.execute(
            "UPDATE work SET current_version_id = ?2, updated_at = ?3 WHERE id = ?1",
            params![work_id, replacement, now()],
        )?;
    }

    tx.commit()?;
    Ok(())
}

struct RawVersion {
    id: String,
    work_id: String,
    role: String,
    revision: i64,
    label: Option<String>,
    body: String,
    meta: String,
    created_at: String,
}

fn read_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<RawVersion> {
    Ok(RawVersion {
        id: row.get(0)?,
        work_id: row.get(1)?,
        role: row.get(2)?,
        revision: row.get(3)?,
        label: row.get(4)?,
        body: row.get(5)?,
        meta: row.get(6)?,
        created_at: row.get(7)?,
    })
}

impl RawVersion {
    fn into_version(self) -> Result<Version> {
        Ok(Version {
            meta: serde_json::from_str(&self.meta)?,
            id: self.id,
            work_id: self.work_id,
            role: self.role,
            revision: self.revision,
            label: self.label,
            body: self.body,
            created_at: self.created_at,
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

    fn a_work(conn: &Connection, profile_id: &str) -> String {
        work::create(
            conn,
            profile_id,
            NewWork {
                kind: "song".into(),
                title: "Subject".into(),
                ..NewWork::default()
            },
        )
        .unwrap()
        .id
    }

    fn draft(role: &str, body: &str) -> NewVersion {
        NewVersion {
            role: role.into(),
            body: body.into(),
            label: None,
            meta: None,
            make_current: true,
        }
    }

    #[test]
    fn revisions_count_up_per_role_independently() {
        let (mut conn, profile_id) = workspace();
        let work_id = a_work(&conn, &profile_id);

        let lyrics_one = create(&mut conn, &work_id, draft("lyrics", "first verse")).unwrap();
        let style_one = create(&mut conn, &work_id, draft("style", "slow, warm")).unwrap();
        let lyrics_two =
            create(&mut conn, &work_id, draft("lyrics", "first verse, fixed")).unwrap();

        assert_eq!(lyrics_one.revision, 1);
        assert_eq!(style_one.revision, 1, "style starts its own count");
        assert_eq!(lyrics_two.revision, 2);
    }

    #[test]
    fn a_new_version_becomes_current_by_default() {
        let (mut conn, profile_id) = workspace();
        let work_id = a_work(&conn, &profile_id);

        let version = create(&mut conn, &work_id, draft("lyrics", "body")).unwrap();

        let work = work::get(&conn, &work_id).unwrap().unwrap();
        assert_eq!(
            work.current_version_id.as_deref(),
            Some(version.id.as_str())
        );
    }

    #[test]
    fn a_version_can_be_added_without_taking_over() {
        let (mut conn, profile_id) = workspace();
        let work_id = a_work(&conn, &profile_id);
        let first = create(&mut conn, &work_id, draft("lyrics", "keep me")).unwrap();

        let mut experiment = draft("lyrics", "an experiment");
        experiment.make_current = false;
        create(&mut conn, &work_id, experiment).unwrap();

        let work = work::get(&conn, &work_id).unwrap().unwrap();
        assert_eq!(work.current_version_id.as_deref(), Some(first.id.as_str()));
    }

    #[test]
    fn bodies_are_stored_whole() {
        let (mut conn, profile_id) = workspace();
        let work_id = a_work(&conn, &profile_id);
        let body = "line one\nline two\nline three";

        let version = create(&mut conn, &work_id, draft("lyrics", body)).unwrap();

        assert_eq!(get(&conn, &version.id).unwrap().unwrap().body, body);
    }

    #[test]
    fn list_marks_the_current_version_and_reports_length() {
        let (mut conn, profile_id) = workspace();
        let work_id = a_work(&conn, &profile_id);
        create(&mut conn, &work_id, draft("lyrics", "short")).unwrap();
        let second = create(&mut conn, &work_id, draft("lyrics", "a longer body")).unwrap();

        let versions = list(&conn, &work_id).unwrap();

        assert_eq!(versions.len(), 2);
        assert_eq!(versions[0].revision, 2, "newest first");
        assert!(versions[0].is_current);
        assert!(!versions[1].is_current);
        assert_eq!(versions[0].length, second.body.chars().count() as i64);
    }

    #[test]
    fn latest_returns_the_highest_revision_of_a_role() {
        let (mut conn, profile_id) = workspace();
        let work_id = a_work(&conn, &profile_id);
        create(&mut conn, &work_id, draft("lyrics", "old")).unwrap();
        create(&mut conn, &work_id, draft("lyrics", "new")).unwrap();
        create(&mut conn, &work_id, draft("style", "unrelated")).unwrap();

        let latest = latest(&conn, &work_id, "lyrics").unwrap().unwrap();

        assert_eq!(latest.body, "new");
    }

    #[test]
    fn set_current_refuses_a_version_from_another_work() {
        let (mut conn, profile_id) = workspace();
        let first = a_work(&conn, &profile_id);
        let second = a_work(&conn, &profile_id);
        let stranger = create(&mut conn, &second, draft("lyrics", "theirs")).unwrap();

        let result = set_current(&conn, &first, &stranger.id);

        assert!(result.is_err());
    }

    #[test]
    fn deleting_the_current_version_falls_back_to_the_previous_one() {
        let (mut conn, profile_id) = workspace();
        let work_id = a_work(&conn, &profile_id);
        let first = create(&mut conn, &work_id, draft("lyrics", "first")).unwrap();
        let second = create(&mut conn, &work_id, draft("lyrics", "second")).unwrap();

        delete(&mut conn, &second.id).unwrap();

        let work = work::get(&conn, &work_id).unwrap().unwrap();
        assert_eq!(
            work.current_version_id.as_deref(),
            Some(first.id.as_str()),
            "the work must not be left pointing at nothing"
        );
    }

    #[test]
    fn deleting_the_only_version_clears_the_pointer() {
        let (mut conn, profile_id) = workspace();
        let work_id = a_work(&conn, &profile_id);
        let only = create(&mut conn, &work_id, draft("lyrics", "alone")).unwrap();

        delete(&mut conn, &only.id).unwrap();

        let work = work::get(&conn, &work_id).unwrap().unwrap();
        assert!(work.current_version_id.is_none());
    }

    #[test]
    fn a_version_needs_an_existing_work() {
        let (mut conn, _) = workspace();

        let result = create(&mut conn, "nope", draft("lyrics", "orphan"));

        assert!(result.is_err());
    }
}
