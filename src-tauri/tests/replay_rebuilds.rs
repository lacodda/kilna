//! The property the operations log exists for: a workspace is the sum of its
//! operations.
//!
//! Playing the log into an empty database has to produce the same rows — not
//! similar ones. A missing operation is invisible when it is not written, and
//! this is the only check that sees it: the rebuilt database simply comes out
//! different.
//!
//! Comparison is over the live tables, read as text, column by column. The log
//! itself is excluded — the replay writes no operations of its own, and a log
//! that recorded its own replay would double on every rebuild.
//!
//! One value is normalised before comparing: a profile's id, which
//! `profile::seed` mints per workspace. Two copies of the same builtin profile
//! carry different ids and the same key, so an id is compared as the key it
//! belongs to. Comparing the raw ids fails on every rebuild while proving
//! nothing about the log — which is what it did the first time this ran.

use rusqlite::Connection;
use rusqlite::types::ValueRef;
use serde_json::{Map, Value};

use kilna_lib::minted::Minted;
use kilna_lib::{db, operation, profile, release, replay, work};

/// Tables a rebuilt workspace has to match on. Everything the person's work
/// lives in; nothing about this machine or this conversation.
const COMPARED: [&str; 9] = [
    "work",
    "work_version",
    "work_score",
    "release",
    "note",
    "collection",
    "work_link",
    "focus_note",
    "tombstone",
];

/// Every profile id in the workspace, against the key that does not move.
fn profile_keys(conn: &Connection) -> Vec<(String, String)> {
    let mut statement = conn.prepare("SELECT id, key FROM profile").unwrap();
    let rows = statement
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
        .unwrap();
    rows.collect::<rusqlite::Result<Vec<(String, String)>>>()
        .unwrap()
}

/// Every compared table, as rows of text, in a fixed order.
fn contents(conn: &Connection) -> Map<String, Value> {
    let mut all = Map::new();
    let profiles = profile_keys(conn);

    for table in COMPARED {
        let mut statement = conn
            .prepare(&format!("SELECT * FROM {table}"))
            .unwrap_or_else(|cause| panic!("{table} is readable: {cause}"));
        let columns: Vec<String> = statement
            .column_names()
            .into_iter()
            .map(str::to_owned)
            .collect();

        let rows = statement
            .query_map([], |row| {
                let mut cells = Vec::new();
                for index in 0..columns.len() {
                    // Read as text rather than through `Debug`: a debug-printed
                    // value shows its bytes, and an id inside it can be neither
                    // recognised nor normalised.
                    let mut cell = match row.get_ref(index)? {
                        ValueRef::Null => "~null".to_owned(),
                        ValueRef::Integer(number) => number.to_string(),
                        ValueRef::Real(number) => number.to_string(),
                        ValueRef::Text(bytes) | ValueRef::Blob(bytes) => {
                            String::from_utf8_lossy(bytes).into_owned()
                        }
                    };
                    // A profile id says which copy this is, not what the work is.
                    for (id, key) in &profiles {
                        if cell == *id {
                            cell.clone_from(key);
                        }
                    }
                    cells.push(cell);
                }
                Ok(cells.join("\u{1f}"))
            })
            .unwrap()
            .collect::<rusqlite::Result<Vec<String>>>()
            .unwrap();

        // Sorted rather than left in insertion order: two databases holding the
        // same rows are the same database, whatever order SQLite hands them
        // back in.
        let mut rows = rows;
        rows.sort();
        all.insert(
            table.to_owned(),
            Value::Array(rows.into_iter().map(Value::String).collect()),
        );
    }

    all
}

/// An empty workspace, seeded exactly as a fresh install is.
fn workspace() -> Connection {
    let conn = db::open_in_memory().unwrap();
    profile::seed(&conn).unwrap();
    conn
}

#[test]
fn a_workspace_is_rebuilt_from_its_operations() {
    let mut source = workspace();
    let profile_id = profile::active(&source).unwrap().unwrap().id;

    // Three works, written the way a command writes them: the operation and the
    // row take the same minted values.
    for title in ["Harbour lights", "Winter road", "The long way round"] {
        let new = work::NewWork {
            kind: "song".into(),
            title: title.into(),
            ..work::NewWork::default()
        };
        let minted = Minted::fresh();
        let logged = operation::Intent::new("work.create")
            .in_profile(&profile_id)
            .param(
                "profile",
                profile::key_for_id(&source, &profile_id).unwrap().unwrap(),
            )
            .param("work", serde_json::to_value(&new).unwrap())
            .minted(&minted);

        let transaction = source.transaction().unwrap();
        work::create_minted(&transaction, &profile_id, new, minted).unwrap();
        operation::record(&transaction, logged).unwrap();
        transaction.commit().unwrap();
    }

    let mut rebuilt = workspace();
    let report = replay::rebuild(&source, &mut rebuilt).unwrap();

    assert_eq!(report.applied, 3, "every operation should have replayed");
    assert!(
        report.unknown.is_empty(),
        "the replay met kinds it does not know: {:?}",
        report.unknown
    );
    assert_eq!(
        contents(&rebuilt),
        contents(&source),
        "the rebuilt workspace is not the same workspace"
    );
}

/// The comparison has to be able to fail.
///
/// Without this, a `contents` that returned nothing — a renamed table, a typo in
/// the list — would make every rebuild look perfect. This project has been
/// caught by that shape of false green before, so the check is asserted rather
/// than assumed.
#[test]
fn the_comparison_notices_a_difference() {
    let mut one = workspace();
    let profile_id = profile::active(&one).unwrap().unwrap().id;

    let two = workspace();

    let new = work::NewWork {
        kind: "song".into(),
        title: "Harbour lights".into(),
        ..work::NewWork::default()
    };
    let transaction = one.transaction().unwrap();
    work::create_minted(&transaction, &profile_id, new, Minted::fresh()).unwrap();
    transaction.commit().unwrap();

    assert_ne!(
        contents(&one),
        contents(&two),
        "a workspace with a work in it reads the same as an empty one — the \
         comparison is not looking at anything"
    );
}

/// An operation this build does not understand is reported, not skipped in
/// silence.
///
/// A log written by a newer version reaches an older one this way. Saying so is
/// the difference between "I could not rebuild all of it" and a database
/// quietly missing whatever those operations did.
#[test]
fn an_unknown_kind_is_reported() {
    let source = workspace();
    operation::record(&source, operation::Intent::new("something.newer")).unwrap();

    let mut rebuilt = workspace();
    let report = replay::rebuild(&source, &mut rebuilt).unwrap();

    assert_eq!(report.applied, 0);
    assert_eq!(report.unknown, vec!["something.newer".to_owned()]);
}

/// A work whose status was *derived* rebuilds with that status, not the one it
/// started with.
///
/// `work.status` is a conclusion drawn from a work's releases and scores, not a
/// decision anyone recorded, so the log does not carry it — and must not, or the
/// same fact would live in two places. That leaves a trap: replaying the rows
/// without recomputing the conclusion gives a database that is right in every
/// table and wrong in the one column a person actually reads.
///
/// This is the test that says whether the replay closes it. It is written as a
/// full circle on purpose — a work, a release, the release going out — because
/// the gap does not exist for a work that was only ever created.
#[test]
fn a_derived_status_survives_the_rebuild() {
    let mut source = workspace();
    let profile_id = profile::active(&source).unwrap().unwrap().id;
    let key = profile::key_for_id(&source, &profile_id).unwrap().unwrap();

    // A work.
    let new = work::NewWork {
        kind: "song".into(),
        title: "Harbour lights".into(),
        ..work::NewWork::default()
    };
    let minted = Minted::fresh();
    let work_id = minted.id().to_owned();
    let logged = operation::Intent::new("work.create")
        .in_profile(&profile_id)
        .param("profile", key.clone())
        .param("work", serde_json::to_value(&new).unwrap())
        .minted(&minted);
    let transaction = source.transaction().unwrap();
    work::create_minted(&transaction, &profile_id, new, minted).unwrap();
    operation::record(&transaction, logged).unwrap();
    transaction.commit().unwrap();

    // A release for it.
    let new = release::NewRelease {
        work_id: work_id.clone(),
        kind: "single".into(),
        title: None,
        scheduled_at: None,
        meta: None,
        scheduled_time: None,
        time_zone: None,
    };
    let minted = Minted::fresh();
    let release_id = minted.id().to_owned();
    let logged = operation::Intent::new("release.create")
        .in_profile(&profile_id)
        .param("profile", key.clone())
        .param("release", serde_json::to_value(&new).unwrap())
        .minted(&minted);
    let transaction = source.transaction().unwrap();
    release::create_minted(&transaction, new, minted).unwrap();
    operation::record(&transaction, logged).unwrap();
    transaction.commit().unwrap();

    // And the release goes out, which is what makes the work "released".
    let at = kilna_lib::time::now();
    let logged = operation::Intent::new("release.markReleased")
        .in_profile(&profile_id)
        .param("profile", key)
        .param("id", release_id.clone())
        .param("at", at.clone());
    let transaction = source.transaction().unwrap();
    release::mark_released_at(&transaction, &release_id, None, None, &at).unwrap();
    operation::record(&transaction, logged).unwrap();
    transaction.commit().unwrap();

    // What the live command does next, and what a rebuild has to reproduce.
    let config = profile::config_for(&source, &profile_id).unwrap();
    work::status::refresh_at(&source, &config, &work_id, &at).unwrap();

    let before = work::get(&source, &work_id).unwrap().unwrap().status;
    assert_eq!(
        before, "released",
        "the fixture is not set up: the work should be released before the rebuild"
    );

    let mut rebuilt = workspace();
    let report = replay::rebuild(&source, &mut rebuilt).unwrap();
    assert!(
        report.unknown.is_empty(),
        "the replay met kinds it does not know: {:?}",
        report.unknown
    );

    let after = work::get(&rebuilt, &work_id).unwrap().unwrap().status;
    assert_eq!(
        after, before,
        "the rebuilt work carries the status it was created with, not the one its \
         released release derives — the replay is not recomputing what the commands do"
    );

    assert_eq!(
        contents(&rebuilt),
        contents(&source),
        "the rebuilt workspace is not the same workspace"
    );
}
