use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::error::{Error, Result};
use crate::minted::Minted;
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
    /// The version this one was written from, when it was written from one.
    /// Revisions stay a line per role for numbering; this is the tree behind
    /// the line. `None` after the parent is deleted — a pruned branch keeps
    /// its leaves.
    pub parent_version_id: Option<String>,
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
    pub parent_version_id: Option<String>,
    pub created_at: String,
    pub is_current: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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
    /// The version this one was derived from. Must belong to the same work
    /// and the same role: a draft is not written from a style prompt.
    #[serde(default)]
    pub parent_version_id: Option<String>,
}

fn default_true() -> bool {
    true
}

const SELECT_VERSION: &str = "SELECT id, work_id, role, revision, label, body, meta, created_at, \
     parent_version_id FROM work_version";

/// Add a version to a work.
///
/// The revision counts up per (work, role), so lyrics and style advance
/// independently — revising a style prompt does not renumber the lyrics.
pub fn create(conn: &mut Connection, work_id: &str, new: NewVersion) -> Result<Version> {
    create_minted(conn, work_id, new, Minted::fresh(), None)
}

/// Add a version with the id and timestamp already decided, recording the
/// operation that asked for it.
///
/// The seam a replay comes back through: live, `create` mints them; replaying,
/// the log supplies what the first run generated, so the version lands under
/// the id everything else already names. See ADR 0014.
///
/// The operation is written inside this function's own transaction rather than
/// by the caller around it, for the reason [`crate::trash::discard_minted`]
/// gives: a log entry committed beside an insert that then failed would replay
/// into a version that never landed.
pub fn create_minted(
    conn: &mut Connection,
    work_id: &str,
    new: NewVersion,
    minted: Minted,
    logged: Option<crate::operation::Intent>,
) -> Result<Version> {
    let tx = conn.transaction()?;

    if let Some(logged) = logged {
        crate::operation::record(&tx, logged)?;
    }

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

    // A parent is a fact about lineage, and lineage does not cross works or
    // roles. Checked here rather than left to the foreign key, which only
    // knows that the row exists.
    if let Some(parent) = &new.parent_version_id {
        let same_line: bool = tx
            .query_row(
                "SELECT 1 FROM work_version WHERE id = ?1 AND work_id = ?2 AND role = ?3",
                params![parent, work_id, new.role],
                |_| Ok(true),
            )
            .optional()?
            .unwrap_or(false);
        if !same_line {
            return Err(Error::Other(format!(
                "version `{parent}` is not a `{}` version of this work, so nothing can be written from it",
                new.role
            )));
        }
    }

    let id = minted.id().to_owned();
    let timestamp = minted.at().to_owned();

    tx.execute(
        "INSERT INTO work_version (id, work_id, role, revision, label, body, meta, created_at,
                                   parent_version_id)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        params![
            id,
            work_id,
            new.role,
            revision,
            new.label,
            new.body,
            Value::Object(new.meta.unwrap_or_default()).to_string(),
            timestamp,
            new.parent_version_id,
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
                v.id = coalesce(w.current_version_id, '') AS is_current, v.parent_version_id
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
            parent_version_id: row.get(8)?,
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

/// Change the body of a version in place.
///
/// The one edit a version accepts, and only while nothing has judged it. The
/// editing session keeps its changes in one version rather than minting one
/// per keystroke or one per opening (ADR 0015) — but a score is a snapshot of
/// the text it read (ADR 0002), and a body rewritten underneath one would
/// silently change what the number was about. A scored version refuses with
/// its own error kind, so the session can start the next revision instead of
/// showing a refusal.
pub fn update_body_at(conn: &Connection, id: &str, body: &str, at: &str) -> Result<Version> {
    let Some(work_id) = conn
        .query_row(
            "SELECT work_id FROM work_version WHERE id = ?1",
            params![id],
            |row| row.get::<_, String>(0),
        )
        .optional()?
    else {
        return Err(Error::not_found("version", id));
    };

    let scored: bool = conn
        .query_row(
            "SELECT 1 FROM work_score WHERE version_id = ?1 LIMIT 1",
            params![id],
            |_| Ok(true),
        )
        .optional()?
        .unwrap_or(false);
    if scored {
        return Err(Error::Frozen(format!(
            "version `{id}` has been scored; a score is a snapshot of the text it read, so the next revision is where this change goes"
        )));
    }

    conn.execute(
        "UPDATE work_version SET body = ?2 WHERE id = ?1",
        params![id, body],
    )?;
    // The work moved too: its summary, its place in "recently edited", the
    // catalogue's sort by change all read `updated_at`.
    conn.execute(
        "UPDATE work SET updated_at = ?2 WHERE id = ?1",
        params![work_id, at],
    )?;

    get(conn, id)?.ok_or_else(|| Error::Other("the version vanished after update".into()))
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
    parent_version_id: Option<String>,
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
        parent_version_id: row.get(8)?,
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
            parent_version_id: self.parent_version_id,
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
            parent_version_id: None,
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
    fn a_body_changes_in_place_until_something_judges_it() {
        let (mut conn, profile_id) = workspace();
        let work_id = a_work(&conn, &profile_id);
        let version = create(&mut conn, &work_id, draft("lyrics", "first line")).unwrap();
        let before = work::get(&conn, &work_id).unwrap().unwrap().updated_at;

        let changed = update_body_at(
            &conn,
            &version.id,
            "first line, then a second",
            "2030-01-01T00:00:00.000Z",
        )
        .unwrap();

        assert_eq!(changed.body, "first line, then a second");
        assert_eq!(
            changed.revision, version.revision,
            "an edit is not a new revision"
        );
        assert_eq!(get(&conn, &version.id).unwrap().unwrap().body, changed.body);
        let after = work::get(&conn, &work_id).unwrap().unwrap().updated_at;
        assert_ne!(after, before, "the work did not register the change");
        assert_eq!(after, "2030-01-01T00:00:00.000Z");
    }

    #[test]
    fn a_scored_version_refuses_to_change_and_says_why() {
        let (mut conn, profile_id) = workspace();
        let work_id = a_work(&conn, &profile_id);
        let version = create(&mut conn, &work_id, draft("lyrics", "judged as is")).unwrap();
        let new_score: crate::score::NewScore =
            serde_json::from_value(serde_json::json!({ "axes": { "hook": 7 } })).unwrap();
        crate::score::create(&conn, &work_id, new_score).unwrap();

        let refused =
            update_body_at(&conn, &version.id, "judged, then changed", &now()).unwrap_err();

        assert_eq!(refused.kind(), "frozen", "{refused}");
        assert_eq!(
            get(&conn, &version.id).unwrap().unwrap().body,
            "judged as is",
            "a refusal must leave the text as the score read it"
        );
    }

    #[test]
    fn a_body_change_is_clocked_on_the_field_it_changed() {
        let (mut conn, profile_id) = workspace();
        let work_id = a_work(&conn, &profile_id);
        let version = create(&mut conn, &work_id, draft("lyrics", "one")).unwrap();

        update_body_at(&conn, &version.id, "two", &now()).unwrap();

        let fields: Vec<String> = conn
            .prepare("SELECT field FROM field_clock WHERE entity = 'work_version' AND entity_id = ?1 ORDER BY field")
            .unwrap()
            .query_map(params![version.id], |row| row.get(0))
            .unwrap()
            .collect::<rusqlite::Result<_>>()
            .unwrap();
        assert_eq!(
            fields,
            vec!["body"],
            "the clock names the field that moved, and only it"
        );
    }

    #[test]
    fn a_version_needs_an_existing_work() {
        let (mut conn, _) = workspace();

        let result = create(&mut conn, "nope", draft("lyrics", "orphan"));

        assert!(result.is_err());
    }

    #[test]
    fn a_version_remembers_the_one_it_was_written_from() {
        let (mut conn, profile_id) = workspace();
        let work_id = a_work(&conn, &profile_id);
        let first = create(&mut conn, &work_id, draft("lyrics", "one")).unwrap();

        let second = create(
            &mut conn,
            &work_id,
            NewVersion {
                parent_version_id: Some(first.id.clone()),
                ..draft("lyrics", "one, revised")
            },
        )
        .unwrap();

        assert_eq!(second.parent_version_id.as_deref(), Some(first.id.as_str()));
        let listed = list(&conn, &work_id).unwrap();
        let summary = listed.iter().find(|v| v.id == second.id).unwrap();
        assert_eq!(
            summary.parent_version_id.as_deref(),
            Some(first.id.as_str())
        );
        assert_eq!(
            listed
                .iter()
                .find(|v| v.id == first.id)
                .unwrap()
                .parent_version_id,
            None
        );
    }

    #[test]
    fn a_parent_must_be_a_version_of_the_same_work_and_role() {
        let (mut conn, profile_id) = workspace();
        let work_id = a_work(&conn, &profile_id);
        let other_work = a_work(&conn, &profile_id);
        let style = create(&mut conn, &work_id, draft("style", "warm")).unwrap();
        let elsewhere = create(&mut conn, &other_work, draft("lyrics", "far")).unwrap();

        for parent in [style.id.clone(), elsewhere.id.clone(), "nothing".to_owned()] {
            let refused = create(
                &mut conn,
                &work_id,
                NewVersion {
                    parent_version_id: Some(parent.clone()),
                    ..draft("lyrics", "child")
                },
            )
            .unwrap_err();
            assert!(
                refused
                    .to_string()
                    .contains("nothing can be written from it"),
                "{parent}: {refused}"
            );
        }
    }

    #[test]
    fn a_pruned_parent_leaves_the_child_without_one_rather_than_gone() {
        let (mut conn, profile_id) = workspace();
        let work_id = a_work(&conn, &profile_id);
        let first = create(&mut conn, &work_id, draft("lyrics", "one")).unwrap();
        let second = create(
            &mut conn,
            &work_id,
            NewVersion {
                parent_version_id: Some(first.id.clone()),
                ..draft("lyrics", "two")
            },
        )
        .unwrap();

        delete(&mut conn, &first.id).unwrap();

        let child = get(&conn, &second.id)
            .unwrap()
            .expect("the child is still there");
        assert_eq!(child.parent_version_id, None);
    }
}
