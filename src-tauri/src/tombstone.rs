//! The trace a deleted row leaves behind.
//!
//! The trash (0003) keeps a snapshot for as long as the person wants it back;
//! this is what stays when the snapshot is gone. A tombstone is written by the
//! delete triggers of migration 0011 — for a row a person deleted, for the
//! rows that went down with it, and for the rows a board or a chat drops
//! without ever visiting the trash — so no code path can forget it.
//!
//! Its job is to make "deleted here" distinguishable from "never arrived
//! here". Today that stops a legacy import from bringing back a work the
//! person threw away; the merge between two devices that the schema is now
//! ready for is built on the same fact.

use rusqlite::{Connection, OptionalExtension, params};
use serde::Serialize;

use crate::error::Result;

/// One deletion, and the restore that may have undone it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Tombstone {
    /// The table the row lived in: `work`, `work_version`, `note`, ...
    pub entity: String,
    pub entity_id: String,
    pub profile_id: Option<String>,
    pub label: Option<String>,
    pub deleted_at: String,
    pub deleted_by: Option<String>,
    pub restored_at: Option<String>,
    pub restored_by: Option<String>,
}

impl Tombstone {
    /// Whether the row is gone as far as this tombstone knows: deleted, and
    /// not restored since.
    pub fn is_buried(&self) -> bool {
        self.restored_at
            .as_deref()
            .is_none_or(|restored| restored < self.deleted_at.as_str())
    }
}

/// The tombstone of one row, if it was ever deleted.
pub fn get(conn: &Connection, entity: &str, entity_id: &str) -> Result<Option<Tombstone>> {
    Ok(conn
        .query_row(
            "SELECT entity, entity_id, profile_id, label, deleted_at, deleted_by,
                    restored_at, restored_by
               FROM tombstone WHERE entity = ?1 AND entity_id = ?2",
            params![entity, entity_id],
            read,
        )
        .optional()?)
}

/// Whether a work with this title was deleted in this profile and never
/// restored.
///
/// A legacy import knows nothing but titles, and this is how it tells a work
/// that is new from one the person already threw out.
pub fn buried_work_title(conn: &Connection, profile_id: &str, title: &str) -> Result<bool> {
    Ok(conn
        .query_row(
            "SELECT 1 FROM tombstone
              WHERE entity = 'work' AND profile_id = ?1 AND label = ?2
                AND restored_at IS NULL
              LIMIT 1",
            params![profile_id, title],
            |_| Ok(true),
        )
        .optional()?
        .unwrap_or(false))
}

fn read(row: &rusqlite::Row<'_>) -> rusqlite::Result<Tombstone> {
    Ok(Tombstone {
        entity: row.get(0)?,
        entity_id: row.get(1)?,
        profile_id: row.get(2)?,
        label: row.get(3)?,
        deleted_at: row.get(4)?,
        deleted_by: row.get(5)?,
        restored_at: row.get(6)?,
        restored_by: row.get(7)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assistant::{self, NewChat};
    use crate::focus::{self, NewFocusNote};
    use crate::note::{self, NewNote};
    use crate::score::{self, NewScore};
    use crate::trash::{self, Entity};
    use crate::work::version::{self, NewVersion};
    use crate::work::{self, NewWork};
    use crate::{db, device, profile, release};
    use serde_json::json;

    fn workspace() -> (Connection, String) {
        let conn = db::open_in_memory().unwrap();
        profile::seed(&conn).unwrap();
        let profile_id = profile::active(&conn).unwrap().unwrap().id;
        (conn, profile_id)
    }

    /// A work with a version, a score, a release and a note hanging off it.
    fn a_full_work(conn: &mut Connection, profile_id: &str) -> (String, [String; 4]) {
        let work = work::create(
            conn,
            profile_id,
            NewWork {
                kind: "song".into(),
                title: "Harbour lights".into(),
                ..NewWork::default()
            },
        )
        .unwrap();
        let version = version::create(
            conn,
            &work.id,
            NewVersion {
                role: "lyrics".into(),
                body: "the cranes go still".into(),
                label: None,
                meta: None,
                make_current: true,
                parent_version_id: None,
            },
        )
        .unwrap();
        let score = score::create(
            conn,
            &work.id,
            NewScore {
                axes: json!({ "hook": 7 }).as_object().cloned().unwrap(),
                version_id: None,
                note: None,
                rater: None,
            },
        )
        .unwrap();
        let planned = release::create(
            conn,
            release::NewRelease {
                work_id: work.id.clone(),
                kind: "clip".into(),
                title: None,
                scheduled_at: None,
                meta: None,
                scheduled_time: None,
                time_zone: None,
            },
        )
        .unwrap();
        let note = note::create(
            conn,
            profile_id,
            NewNote {
                body: "ask about the artwork".into(),
                kind: None,
                title: None,
                work_id: Some(work.id.clone()),
                tags: Vec::new(),
            },
        )
        .unwrap();
        (work.id, [version.id, score.id, planned.id, note.id])
    }

    fn buried(conn: &Connection, entity: &str, id: &str) -> bool {
        get(conn, entity, id)
            .unwrap()
            .is_some_and(|t| t.is_buried())
    }

    #[test]
    fn a_row_that_was_never_deleted_has_no_tombstone() {
        let (mut conn, profile_id) = workspace();
        let (work_id, _) = a_full_work(&mut conn, &profile_id);
        assert_eq!(get(&conn, "work", &work_id).unwrap(), None);
    }

    #[test]
    fn deleting_a_work_buries_it_and_everything_beneath_it() {
        let (mut conn, profile_id) = workspace();
        let (work_id, [version_id, score_id, release_id, note_id]) =
            a_full_work(&mut conn, &profile_id);

        trash::discard(&mut conn, Entity::Work, &work_id).unwrap();

        let stone = get(&conn, "work", &work_id).unwrap().expect("a tombstone");
        assert!(stone.is_buried());
        assert_eq!(stone.profile_id.as_deref(), Some(profile_id.as_str()));
        assert_eq!(stone.label.as_deref(), Some("Harbour lights"));
        assert_eq!(stone.deleted_by, device::id(&conn).unwrap());
        assert_eq!(stone.restored_at, None);

        // The cascade reached them through the schema, not through the trash,
        // and the trigger caught each one on its own table.
        assert!(buried(&conn, "work_version", &version_id));
        assert!(buried(&conn, "work_score", &score_id));
        assert!(buried(&conn, "release", &release_id));
        assert!(buried(&conn, "note", &note_id));
    }

    #[test]
    fn restoring_marks_the_tombstone_rather_than_removing_it() {
        let (mut conn, profile_id) = workspace();
        let (work_id, [version_id, ..]) = a_full_work(&mut conn, &profile_id);
        let entry = trash::discard(&mut conn, Entity::Work, &work_id).unwrap();

        trash::restore(&mut conn, &entry).unwrap();

        let stone = get(&conn, "work", &work_id).unwrap().expect("kept");
        assert!(!stone.is_buried());
        assert!(stone.restored_at.is_some());
        assert_eq!(stone.restored_by, device::id(&conn).unwrap());
        // The children came back with it and were marked the same way.
        assert!(!buried(&conn, "work_version", &version_id));
    }

    #[test]
    fn deleting_again_after_a_restore_starts_the_cycle_over() {
        let (mut conn, profile_id) = workspace();
        let (work_id, _) = a_full_work(&mut conn, &profile_id);
        let entry = trash::discard(&mut conn, Entity::Work, &work_id).unwrap();
        trash::restore(&mut conn, &entry).unwrap();

        trash::discard(&mut conn, Entity::Work, &work_id).unwrap();

        let stone = get(&conn, "work", &work_id).unwrap().unwrap();
        assert!(stone.is_buried());
        assert_eq!(
            stone.restored_at, None,
            "a fresh deletion, not a stale restore"
        );
    }

    #[test]
    fn emptying_the_trash_does_not_touch_the_tombstone() {
        let (mut conn, profile_id) = workspace();
        let (work_id, _) = a_full_work(&mut conn, &profile_id);
        let entry = trash::discard(&mut conn, Entity::Work, &work_id).unwrap();

        trash::purge(&mut conn, &entry).unwrap();
        assert!(buried(&conn, "work", &work_id), "purged, still buried");

        let (other_id, _) = a_full_work(&mut conn, &profile_id);
        trash::discard(&mut conn, Entity::Work, &other_id).unwrap();
        trash::empty(&conn, &profile_id).unwrap();
        assert!(buried(&conn, "work", &other_id), "emptied, still buried");
    }

    #[test]
    fn what_never_visits_the_trash_is_buried_all_the_same() {
        let (conn, profile_id) = workspace();
        let chat = assistant::create(
            &conn,
            &profile_id,
            NewChat {
                work_id: None,
                title: Some("About the bridge".into()),
            },
        )
        .unwrap();
        let line = focus::add_note(
            &conn,
            &profile_id,
            NewFocusNote {
                body: "ask the label about the artwork".into(),
                work_id: None,
                due_on: None,
            },
        )
        .unwrap();

        assistant::delete(&conn, &chat.id).unwrap();
        focus::delete_note(&conn, &line.id).unwrap();

        let stone = get(&conn, "chat", &chat.id).unwrap().expect("chat buried");
        assert_eq!(stone.label.as_deref(), Some("About the bridge"));
        assert_eq!(stone.profile_id.as_deref(), Some(profile_id.as_str()));
        let stone = get(&conn, "focus_note", &line.id)
            .unwrap()
            .expect("note buried");
        assert_eq!(
            stone.label.as_deref(),
            Some("ask the label about the artwork")
        );
    }

    #[test]
    fn a_buried_title_is_known_by_its_profile_and_forgotten_on_restore() {
        let (mut conn, profile_id) = workspace();
        let (work_id, _) = a_full_work(&mut conn, &profile_id);
        assert!(!buried_work_title(&conn, &profile_id, "Harbour lights").unwrap());

        let entry = trash::discard(&mut conn, Entity::Work, &work_id).unwrap();
        assert!(buried_work_title(&conn, &profile_id, "Harbour lights").unwrap());
        assert!(!buried_work_title(&conn, "another-profile", "Harbour lights").unwrap());
        assert!(!buried_work_title(&conn, &profile_id, "Paper boats").unwrap());

        trash::restore(&mut conn, &entry).unwrap();
        assert!(!buried_work_title(&conn, &profile_id, "Harbour lights").unwrap());
    }
}
