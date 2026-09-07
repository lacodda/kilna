//! When each field of a row last changed, and where.
//!
//! The clocks are written by the triggers of migration 0011, one row per field
//! that actually changed; this module only reads them. A row's `updated_at`
//! says that *something* changed — the clocks say what, which is the difference
//! between two devices overwriting each other whole and each keeping the field
//! it edited. Nothing in the application consults them yet: they exist so that
//! the day a merge is written, every edit made before it already has a history.

use rusqlite::{Connection, params};
use serde::Serialize;

use crate::error::Result;

/// One field's last change.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FieldClock {
    pub field: String,
    pub changed_at: String,
    pub device_id: Option<String>,
}

/// Every clocked field of one row, in field order.
pub fn for_row(conn: &Connection, entity: &str, entity_id: &str) -> Result<Vec<FieldClock>> {
    let mut statement = conn.prepare(
        "SELECT field, changed_at, device_id FROM field_clock
          WHERE entity = ?1 AND entity_id = ?2 ORDER BY field",
    )?;
    let rows = statement.query_map(params![entity, entity_id], |row| {
        Ok(FieldClock {
            field: row.get(0)?,
            changed_at: row.get(1)?,
            device_id: row.get(2)?,
        })
    })?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::work::{self, NewWork, WorkPatch};
    use crate::{db, device, profile, release, trash};
    use serde_json::json;

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
                title: "Harbour lights".into(),
                ..NewWork::default()
            },
        )
        .unwrap()
        .id
    }

    fn fields(conn: &Connection, id: &str) -> Vec<String> {
        for_row(conn, "work", id)
            .unwrap()
            .into_iter()
            .map(|clock| clock.field)
            .collect()
    }

    fn retitle(conn: &Connection, id: &str, title: &str) {
        work::update(
            conn,
            id,
            WorkPatch {
                title: Some(title.into()),
                ..WorkPatch::default()
            },
        )
        .unwrap();
    }

    #[test]
    fn creating_a_row_writes_no_clocks() {
        let (conn, profile_id) = workspace();
        let id = a_work(&conn, &profile_id);
        assert!(
            fields(&conn, &id).is_empty(),
            "created_at is the clock of a new row"
        );
    }

    #[test]
    fn only_the_field_that_changed_gets_a_clock() {
        let (conn, profile_id) = workspace();
        let id = a_work(&conn, &profile_id);

        retitle(&conn, &id, "Harbour lights (edit)");

        assert_eq!(fields(&conn, &id), vec!["title"]);
    }

    #[test]
    fn a_save_with_the_same_values_leaves_no_mark() {
        let (conn, profile_id) = workspace();
        let id = a_work(&conn, &profile_id);

        retitle(&conn, &id, "Harbour lights");

        assert!(fields(&conn, &id).is_empty());
    }

    #[test]
    fn a_clock_names_this_device_and_takes_the_stored_stamp_shape() {
        let (conn, profile_id) = workspace();
        let id = a_work(&conn, &profile_id);
        // Not the status: a status set by hand pins itself, which is two
        // fields changing at once and a different test's business.
        work::update(
            &conn,
            &id,
            WorkPatch {
                kind: Some("poem".into()),
                ..WorkPatch::default()
            },
        )
        .unwrap();

        let clocks = for_row(&conn, "work", &id).unwrap();
        let [clock] = clocks.as_slice() else {
            panic!("one clock expected, got {clocks:?}");
        };
        assert_eq!(clock.field, "kind");
        assert_eq!(clock.device_id, device::id(&conn).unwrap());
        // `2026-09-07T10:11:12.345Z`, exactly as `time::now()` writes it, so
        // the two kinds of stamp sort together.
        assert_eq!(clock.changed_at.len(), 24, "{}", clock.changed_at);
        assert!(clock.changed_at.ends_with('Z'));
        assert_eq!(&clock.changed_at[19..20], ".");
        assert!(clock.changed_at <= crate::time::now());
    }

    #[test]
    fn a_later_change_replaces_the_clock_rather_than_adding_one() {
        let (conn, profile_id) = workspace();
        let id = a_work(&conn, &profile_id);
        retitle(&conn, &id, "One");
        retitle(&conn, &id, "Two");
        assert_eq!(fields(&conn, &id), vec!["title"]);
    }

    #[test]
    fn a_field_going_to_or_from_null_is_a_change() {
        let (mut conn, profile_id) = workspace();
        let id = a_work(&conn, &profile_id);
        let planned = release::create(
            &conn,
            release::NewRelease {
                work_id: id.clone(),
                kind: "clip".into(),
                title: None,
                scheduled_at: None,
                meta: None,
            },
        )
        .unwrap();

        release::schedule(&mut conn, &planned.id, "2026-10-01").unwrap();
        let after_schedule = for_row(&conn, "release", &planned.id).unwrap();
        assert!(
            after_schedule
                .iter()
                .any(|clock| clock.field == "scheduled_at"),
            "{after_schedule:?}"
        );

        release::unschedule(&conn, &planned.id).unwrap();
        let clocks = for_row(&conn, "release", &planned.id).unwrap();
        assert!(clocks.iter().any(|clock| clock.field == "scheduled_at"));
        assert!(
            !clocks.iter().any(|clock| clock.field == "slot_pinned_at"),
            "null to null is not a change: {clocks:?}"
        );
    }

    #[test]
    fn deleting_a_row_takes_its_clocks_with_it() {
        let (mut conn, profile_id) = workspace();
        let id = a_work(&conn, &profile_id);
        work::update(
            &conn,
            &id,
            WorkPatch {
                meta: Some(json!({ "bpm": 120 }).as_object().cloned().unwrap()),
                ..WorkPatch::default()
            },
        )
        .unwrap();
        assert_eq!(fields(&conn, &id), vec!["meta"]);

        trash::discard(&mut conn, trash::Entity::Work, &id).unwrap();

        assert!(fields(&conn, &id).is_empty());
    }

    #[test]
    fn a_machine_local_column_is_not_clocked() {
        let (conn, _) = workspace();
        let count = || -> i64 {
            conn.query_row("SELECT count(*) FROM field_clock", [], |row| row.get(0))
                .unwrap()
        };
        let before = count();

        // Which profile is active is a fact about this machine, not the work.
        conn.execute("UPDATE profile SET is_active = 0 WHERE is_active = 1", [])
            .unwrap();

        assert_eq!(count(), before);
    }
}
