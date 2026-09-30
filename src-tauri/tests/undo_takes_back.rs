//! Undo puts back what the last operation changed — and nothing else.
//!
//! The interesting failures here are not "nothing happened". They are the ones
//! where something happened and it was slightly too much: an undo that reverts
//! a field nobody touched, or that quietly reverses the wrong operation because
//! the offer went stale. Each has its own test below.
//!
//! One of them needs the log itself rather than the row. Making `invert` build
//! its answer from the whole row instead of from the patch leaves every
//! assertion about rows green: `WorkPatch` is deserialised with serde, which
//! drops the keys it does not know, so an inverse carrying `id`, `created_at`
//! and the rest arrives at the domain looking exactly like a correct one. The
//! damage is real but invisible from here — it is a `before` that claims to
//! describe fields the person never edited, and the next kind of patch that
//! does know one of those keys will act on it. So the shape of what was
//! recorded is asserted directly, in `the_recorded_before_names_only_the_edit`.

mod common;

use rusqlite::Connection;

use kilna_lib::actions;
use kilna_lib::minted::Minted;
use kilna_lib::work::version;
use kilna_lib::work::{NewWork, WorkPatch};
use kilna_lib::{db, fixtures, operation, profile, undo, work};

/// A workspace with one work in it, and the id of that work.
fn workspace() -> (Connection, String, String) {
    let (conn, profile_id) = fixtures::workspace();
    let work_id = actions::work::create(
        &conn,
        NewWork {
            kind: "song".into(),
            title: "Harbour lights".into(),
            ..NewWork::default()
        },
    )
    .unwrap()
    .id;
    (conn, profile_id, work_id)
}

/// Take back the operation on offer, and hold the undo to the log.
///
/// Every undo here goes through this, so every kind these tests take back is
/// also checked to have left its own line in the operations log - the undo is
/// a change like any other, and a log without it replays to a database that
/// still holds what was taken back. Until v0.77 that was a check that
/// `undo.rs` mentioned `operation::record` somewhere in its text.
fn take_back(conn: &mut Connection, offer: &undo::Undoable) {
    let taken = operation::by_id(conn, &offer.operation_id)
        .unwrap()
        .expect("the offer names an operation in the log");
    let before = operation::count(conn).unwrap();

    undo::undo(conn, &offer.operation_id).unwrap();

    assert_eq!(
        operation::count(conn).unwrap(),
        before + 1,
        "taking back `{}` left no line of its own in the operations log",
        taken.kind
    );
    let written = operation::latest(conn, 1).unwrap().remove(0);
    assert_eq!(written.kind, format!("{}{}", undo::UNDO_PREFIX, taken.kind));
    assert_eq!(
        written.params.get("operation").and_then(|id| id.as_str()),
        Some(taken.id.as_str()),
        "the undo's line does not say which operation it took back"
    );
}

/// Edit a work the way the command does: `actions::work::update`, exactly.
fn edit(conn: &mut Connection, _profile_id: &str, work_id: &str, patch: WorkPatch) {
    actions::work::update(conn, work_id, patch).unwrap();
}

#[test]
fn an_edit_is_taken_back() {
    let (mut conn, profile_id, work_id) = workspace();

    edit(
        &mut conn,
        &profile_id,
        &work_id,
        WorkPatch {
            title: Some("Winter road".into()),
            ..WorkPatch::default()
        },
    );
    assert_eq!(
        work::get(&conn, &work_id).unwrap().unwrap().title,
        "Winter road"
    );

    let offer = undo::last(&conn).unwrap().expect("the edit can be undone");
    assert_eq!(offer.action, "undo.work.update");
    take_back(&mut conn, &offer);

    assert_eq!(
        work::get(&conn, &work_id).unwrap().unwrap().title,
        "Harbour lights",
        "the title did not come back"
    );
}

/// The property that separates a real undo from a plausible one.
///
/// Undoing a rename must leave alone a status somebody set in between. This is
/// the failure that would never show up in a demo and would cost real work.
#[test]
fn undoing_one_field_leaves_another_alone() {
    let (mut conn, profile_id, work_id) = workspace();

    edit(
        &mut conn,
        &profile_id,
        &work_id,
        WorkPatch {
            title: Some("Winter road".into()),
            ..WorkPatch::default()
        },
    );
    edit(
        &mut conn,
        &profile_id,
        &work_id,
        WorkPatch {
            status: Some("shelved".into()),
            ..WorkPatch::default()
        },
    );

    // Take back the status change; the rename must survive.
    let offer = undo::last(&conn).unwrap().unwrap();
    take_back(&mut conn, &offer);

    let after = work::get(&conn, &work_id).unwrap().unwrap();
    assert_eq!(after.status, "draft", "the status did not come back");
    assert_eq!(
        after.title, "Winter road",
        "undoing the status change also reverted the rename — the inverse is \
         being built from the whole row instead of from the patch"
    );
}

/// A patch naming these fields of the work's, and nothing else.
fn fields(value: serde_json::Value) -> WorkPatch {
    WorkPatch {
        meta: value.as_object().cloned(),
        ..WorkPatch::default()
    }
}

/// The work's fields, as the row holds them now.
fn meta_of(conn: &Connection, work_id: &str) -> serde_json::Value {
    serde_json::Value::Object(work::get(conn, work_id).unwrap().unwrap().meta)
}

/// Two fields edited one after the other, and the second is taken back: the
/// first keeps its new value. The log's `before` names the one field the edit
/// sent, not the object it sat in.
#[test]
fn undoing_one_field_of_a_work_leaves_the_field_edited_before_it() {
    let (mut conn, profile_id, work_id) = workspace();

    edit(
        &mut conn,
        &profile_id,
        &work_id,
        fields(serde_json::json!({ "bpm": 92 })),
    );
    edit(
        &mut conn,
        &profile_id,
        &work_id,
        fields(serde_json::json!({ "key": "Am" })),
    );
    assert_eq!(
        meta_of(&conn, &work_id),
        serde_json::json!({ "bpm": 92, "key": "Am" })
    );

    let entry = operation::latest(&conn, 1).unwrap().pop().unwrap();
    assert_eq!(
        entry.params["before"],
        serde_json::json!({ "meta": { "key": null } }),
        "`before` should name the key the edit sent - and as absent, so the undo removes it"
    );

    let offer = undo::last(&conn)
        .unwrap()
        .expect("a field edit can be undone");
    assert_eq!(offer.action, "undo.work.update");
    take_back(&mut conn, &offer);

    assert_eq!(
        meta_of(&conn, &work_id),
        serde_json::json!({ "bpm": 92 }),
        "the key went, and the tempo edited before it stayed"
    );
}

/// The first of two field edits is taken back while the second - made
/// meanwhile by something that is not on the undo stack, a second window or
/// an arriving sync - stays. With the whole object in `before`, as until
/// v0.82, the undo put the neighbour back to what it was before the edit.
#[test]
fn undoing_one_field_leaves_a_neighbour_changed_since() {
    let (mut conn, profile_id, work_id) = workspace();
    edit(
        &mut conn,
        &profile_id,
        &work_id,
        fields(serde_json::json!({ "bpm": 128, "key": "Am" })),
    );

    edit(
        &mut conn,
        &profile_id,
        &work_id,
        fields(serde_json::json!({ "bpm": 92 })),
    );
    let offer = undo::last(&conn).unwrap().unwrap();
    work::update(&conn, &work_id, fields(serde_json::json!({ "key": "Em" }))).unwrap();

    take_back(&mut conn, &offer);

    assert_eq!(
        meta_of(&conn, &work_id),
        serde_json::json!({ "bpm": 128, "key": "Em" }),
        "the tempo came back and the key changed since stayed"
    );
}

/// An edit logged before v0.82 held the whole object on both sides. Taken
/// back under the merge it still puts the work back as it was - the field it
/// added removed again, the one it dropped restored.
#[test]
fn a_whole_object_edit_from_an_older_log_is_taken_back_whole() {
    let (mut conn, profile_id, work_id) = workspace();
    work::update(
        &conn,
        &work_id,
        fields(serde_json::json!({ "bpm": 128, "key": "Am" })),
    )
    .unwrap();

    // What the screen sent until v0.82 to change the tempo, drop the key and
    // add a mood: every field the work was to be left with.
    let key = profile::key_for_id(&conn, &profile_id).unwrap().unwrap();
    let at = kilna_lib::time::now();
    let logged = operation::Intent::new("work.update")
        .in_profile(&profile_id)
        .param("profile", key)
        .param("id", work_id.clone())
        .param(
            "patch",
            serde_json::json!({ "meta": { "bpm": 92, "mood": "dark" } }),
        )
        .param(
            "before",
            serde_json::json!({ "meta": { "bpm": 128, "key": "Am" } }),
        )
        .param("at", at.clone());
    let transaction = conn.transaction().unwrap();
    work::update_at(
        &transaction,
        &work_id,
        fields(serde_json::json!({ "bpm": 92, "key": null, "mood": "dark" })),
        &at,
    )
    .unwrap();
    operation::record(&transaction, logged).unwrap();
    transaction.commit().unwrap();

    let offer = undo::last(&conn).unwrap().unwrap();
    take_back(&mut conn, &offer);

    assert_eq!(
        meta_of(&conn, &work_id),
        serde_json::json!({ "bpm": 128, "key": "Am" })
    );
}

/// An undo is not itself undoable.
///
/// Offering to take back an undo would be a redo wearing the same button, and
/// pressing undo twice would then oscillate rather than walk back.
#[test]
fn an_undo_cannot_itself_be_undone() {
    let (mut conn, profile_id, work_id) = workspace();

    edit(
        &mut conn,
        &profile_id,
        &work_id,
        WorkPatch {
            title: Some("Winter road".into()),
            ..WorkPatch::default()
        },
    );
    let offer = undo::last(&conn).unwrap().unwrap();
    take_back(&mut conn, &offer);

    assert!(
        undo::last(&conn).unwrap().is_none(),
        "the undo is being offered as something to undo"
    );
}

/// Undoing an operation that is no longer the last one is refused.
///
/// The offer is read once and pressed later; in between, a background sweep or
/// a second window may have written. Reversing regardless would take back
/// something the person did not mean.
#[test]
fn a_stale_offer_is_refused() {
    let (mut conn, profile_id, work_id) = workspace();

    edit(
        &mut conn,
        &profile_id,
        &work_id,
        WorkPatch {
            title: Some("Winter road".into()),
            ..WorkPatch::default()
        },
    );
    let stale = undo::last(&conn).unwrap().unwrap();

    edit(
        &mut conn,
        &profile_id,
        &work_id,
        WorkPatch {
            title: Some("The long way round".into()),
            ..WorkPatch::default()
        },
    );

    let refused = undo::undo(&conn, &stale.operation_id);

    assert!(refused.is_err(), "a stale undo was carried out anyway");
    assert_eq!(
        work::get(&conn, &work_id).unwrap().unwrap().title,
        "The long way round",
        "the refused undo changed the row anyway"
    );
}

/// The `before` written into the log names the edited fields and no others.
///
/// Asserted on the log rather than on the row because the row cannot tell:
/// serde drops unknown keys when the inverse is read back into a patch, so a
/// `before` describing the whole work behaves identically — until a patch type
/// gains a field that was riding along unnoticed. Verified by mutation: making
/// `invert` iterate the row instead of the patch leaves every other test in
/// this file green and fails this one.
#[test]
fn the_recorded_before_names_only_the_edit() {
    let (mut conn, profile_id, work_id) = workspace();

    edit(
        &mut conn,
        &profile_id,
        &work_id,
        WorkPatch {
            title: Some("Winter road".into()),
            ..WorkPatch::default()
        },
    );

    let entry = operation::latest(&conn, 1).unwrap().pop().unwrap();
    assert_eq!(entry.kind, "work.update");
    let before = entry.params["before"]
        .as_object()
        .expect("a `before` object");

    let named: Vec<&String> = before.keys().collect();
    assert_eq!(
        named,
        vec!["title"],
        "`before` should describe the one field the patch touched, not {named:?}"
    );
    assert_eq!(before["title"], "Harbour lights");
}

/// Undoing a creation puts the thing in the trash, not out of existence.
///
/// The difference matters: kilna never destroys a row on one gesture, and an
/// undo that did would be the only place in the application where changing your
/// mind twice is impossible.
#[test]
fn undoing_a_creation_sends_it_to_the_trash() {
    let (mut conn, profile_id, work_id) = workspace();

    let offer = undo::last(&conn)
        .unwrap()
        .expect("a creation can be undone");
    assert_eq!(offer.action, "undo.work.create");
    take_back(&mut conn, &offer);

    assert!(
        work::get(&conn, &work_id).unwrap().is_none(),
        "the work is still there"
    );
    let waiting = kilna_lib::trash::list(&conn, &profile_id).unwrap();
    assert_eq!(
        waiting.len(),
        1,
        "the work was destroyed rather than moved to the trash"
    );
    assert_eq!(waiting[0].entity_id, work_id);
}

/// Undoing a deletion brings the row back under the same id.
///
/// Under the *same* id, not an equivalent row: everything that pointed at it —
/// versions, scores, releases — has to still point at it afterwards.
#[test]
fn undoing_a_deletion_brings_the_row_back() {
    let (mut conn, _profile_id, work_id) = workspace();

    kilna_lib::actions::trash::discard(&conn, kilna_lib::trash::Entity::Work, &work_id).unwrap();
    assert!(work::get(&conn, &work_id).unwrap().is_none());

    let offer = undo::last(&conn)
        .unwrap()
        .expect("a deletion can be undone");
    take_back(&mut conn, &offer);

    let back = work::get(&conn, &work_id).unwrap();
    assert!(back.is_some(), "the work did not come back");
    assert_eq!(
        back.unwrap().title,
        "Harbour lights",
        "the work came back as something else"
    );
}

/// A link made is taken back by removing it; a link removed is taken back by
/// putting the same row back — same id, same moment, same version taken.
#[test]
fn a_link_is_taken_back_both_ways() {
    let (mut conn, profile_id, song_id) = workspace();
    let video_id = fixtures::video(&conn, &profile_id, "Harbour lights").id;

    let new = kilna_lib::link::NewLink {
        work_id: video_id.clone(),
        source_id: song_id.clone(),
        role: None,
        source_version_id: None,
    };
    let link_id = actions::link::create(&conn, new).unwrap().id;

    let offer = undo::last(&conn)
        .unwrap()
        .expect("a link made can be undone");
    assert_eq!(offer.action, "undo.link.create");
    take_back(&mut conn, &offer);
    assert!(
        kilna_lib::link::get(&conn, &link_id).unwrap().is_none(),
        "the link is still there"
    );

    // Made again, then removed the way the command removes it, then undone.
    let made = actions::link::create(
        &conn,
        kilna_lib::link::NewLink {
            work_id: video_id.clone(),
            source_id: song_id.clone(),
            role: None,
            source_version_id: None,
        },
    )
    .unwrap();
    let link_id = made.id.clone();
    actions::link::delete(&conn, &link_id).unwrap();
    assert!(kilna_lib::link::get(&conn, &link_id).unwrap().is_none());

    let offer = undo::last(&conn)
        .unwrap()
        .expect("a link removed can be undone");
    assert_eq!(offer.action, "undo.link.delete");
    take_back(&mut conn, &offer);

    let back = kilna_lib::link::get(&conn, &link_id)
        .unwrap()
        .expect("the link came back");
    assert_eq!(back.created_at, made.created_at, "under its own moment");
    assert_eq!(back.source_id, song_id);
    assert_eq!(back.role, "donor");
}

/// An operation kind that is not reversible is not offered.
///
/// The list in `undo::reversible` is the promise; this checks the promise is
/// kept rather than being a comment. `status.resync` is the case: it recomputes
/// many works from the facts, and putting them all back where they were is not
/// something one stored `before` can describe.
#[test]
fn an_irreversible_operation_is_not_offered() {
    let (conn, profile_id, _work_id) = workspace();

    let key = profile::key_for_id(&conn, &profile_id).unwrap().unwrap();
    let logged = operation::Intent::new("status.resync")
        .in_profile(&profile_id)
        .param("profile", key)
        .param("at", kilna_lib::time::now());
    operation::record(&conn, logged).unwrap();

    assert!(
        undo::last(&conn).unwrap().is_none(),
        "a resync is being offered as undoable, and nothing knows how to take it back"
    );
    assert!(!undo::reversible("status.resync"));
}

/// Every kind of operation is either undoable or has a reason why not.
///
/// The list below is the promise made to the person who presses the key: this
/// is what comes back and this is what does not, and none of it is decided by
/// whatever the `match` in `undo::reverse` happens to cover. A kind added to
/// the log without being sorted into one of the two fails here, so the
/// question cannot be skipped by forgetting it.
#[test]
fn every_operation_is_undoable_or_says_why_not() {
    // The reason is not read by the code — it is read by whoever runs into this
    // test after adding a kind, which is exactly when it needs to exist.
    const NOT_UNDOABLE: [(&str, &str); 25] = [
        (
            "style.attachReference",
            "it carries a file into the workspace, as `asset.attach` does, and \
             taking it back is detaching the asset, which removes the bytes too",
        ),
        (
            "status.resync",
            "recomputes many works from the facts at once; the statuses it \
             replaced are not one `before` a single operation can hold",
        ),
        (
            "work.setStatusBatch",
            "one gesture over many works, each with its own previous status; \
             taking it back needs a batch inverse, which is its own stage",
        ),
        (
            "work.discardBatch",
            "as above, and each work has its own trash entry",
        ),
        ("release.unscheduleBatch", "as above"),
        (
            "work.unpinStatus",
            "handing a status back to the automation immediately recomputes it; \
             re-pinning would put back the word without the reason it was pinned",
        ),
        ("work.unpinTier", "as above, for the tier"),
        (
            "release.unmarkReleased",
            "the operation does not carry the link or the day it took away, so \
             an undo could only put back half of it — see the session of \
             2026-09-09; it becomes undoable when the operation carries them",
        ),
        (
            "release.schedule",
            "a slot is taken from the plan the calendar computed; putting the \
             release back into a queue that has since moved on is a different \
             question from undoing",
        ),
        ("release.unschedule", "as above"),
        (
            "release.setSlotPin",
            "the pin is one click to set again, and the calendar shows it",
        ),
        (
            "layout.apply",
            "places many releases at once; a batch inverse, as above",
        ),
        (
            "collection.setContents",
            "replaces a whole membership list; the previous list is not recorded",
        ),
        (
            "score.create",
            "a score is a snapshot on purpose (ADR 0002) — it is added, never replaced",
        ),
        (
            "focusNote.create",
            "the board is a scratch surface; its notes are one click to remove",
        ),
        ("focusNote.update", "as above"),
        ("focusNote.delete", "as above"),
        ("focusNote.reorder", "as above"),
        (
            "finding.dismiss",
            "the board offers `restore_finding` in place, which is the undo",
        ),
        ("finding.restore", "as above"),
        (
            "trash.restore",
            "restoring is itself the undo of a deletion; undoing it would be a redo",
        ),
        (
            "asset.attach",
            "the copy it made would have to be unmade, and the row is the only \
             thing the log holds — taking it back is `detach_asset`, which \
             removes the bytes as well",
        ),
        (
            "asset.detach",
            "the bytes are gone with the row; a row put back beside a file that \
             no longer exists is the broken picture the deletion avoided",
        ),
        (
            "scene.attachFrame",
            "it carries a file into the workspace, as `asset.attach` does, and \
             taking it back is `detach_scene_frame`, which removes the bytes too",
        ),
        (
            "scene.detachFrame",
            "the picture is gone with the row; putting the row back beside a \
             file that no longer exists is the broken frame the deletion avoided",
        ),
    ];

    // Kinds handled by the trash rather than by an operation of their own.
    const HANDLED_ELSEWHERE: [&str; 2] = ["trash.purge", "trash.empty"];

    // Every operation the backend writes, wherever it is written: the kind a
    // gesture records, and any intent still built by hand. Read from the whole
    // backend rather than one file, so an action moved to a file of its own is
    // still read.
    let sources = common::backend();
    let mut kinds: Vec<String> = Vec::new();
    let gestured = sources
        .iter()
        .flat_map(common::gestures)
        .filter_map(|(_, kind)| kind);
    let by_hand = common::literal_arguments(&sources, "Intent::new")
        .into_iter()
        .map(|(_, kind)| kind);
    for kind in gestured.chain(by_hand) {
        if !kinds.contains(&kind) {
            kinds.push(kind);
        }
    }

    assert!(
        kinds.len() > 20,
        "only {} kinds found — the scan has stopped seeing the operations",
        kinds.len()
    );

    let unsorted: Vec<&String> = kinds
        .iter()
        .filter(|kind| !undo::reversible(kind))
        .filter(|kind| !NOT_UNDOABLE.iter().any(|(named, _)| named == *kind))
        .filter(|kind| !HANDLED_ELSEWHERE.contains(&kind.as_str()))
        .collect();

    assert!(
        unsorted.is_empty(),
        "these operations are neither undoable nor listed as not: {unsorted:?}\n\
         Add each to `undo::reversible` with a reversal, or to NOT_UNDOABLE with \
         the reason a person cannot take it back."
    );

    // And the list may not name something that has quietly become undoable.
    let stale: Vec<&str> = NOT_UNDOABLE
        .iter()
        .map(|(kind, _)| *kind)
        .filter(|kind| undo::reversible(kind))
        .collect();
    assert!(
        stale.is_empty(),
        "these are listed as impossible to undo but `undo::reversible` says otherwise: {stale:?}"
    );
}

/// A version's body goes back whole, and the revision number does not move.
#[test]
fn a_body_edit_is_taken_back() {
    let (mut conn, _profile_id, work_id) = workspace();
    let version = fixtures::version(&conn, &work_id, "lyrics", "as written");

    actions::version::edit(&conn, &version.id, "as rewritten").unwrap();

    let offer = undo::last(&conn).unwrap().expect("the edit can be undone");
    assert_eq!(offer.action, "undo.version.edit");
    take_back(&mut conn, &offer);

    let back = version::get(&conn, &version.id).unwrap().unwrap();
    assert_eq!(back.body, "as written", "the text did not come back");
    assert_eq!(back.revision, version.revision);
}

/// Choosing the current version is an edit of the work, and Ctrl+Z takes back
/// that choice. It used to be written past the log, so the undo offered after
/// it was the edit before it.
#[test]
fn choosing_the_current_version_is_what_undo_takes_back() {
    let (mut conn, profile_id, work_id) = workspace();
    let first = fixtures::version(&conn, &work_id, "lyrics", "first");
    let second = fixtures::version(&conn, &work_id, "lyrics", "second");
    assert_eq!(
        work::get(&conn, &work_id)
            .unwrap()
            .unwrap()
            .current_version_id
            .as_deref(),
        Some(second.id.as_str())
    );

    // What `set_current_version` records since v0.76.1.
    edit(
        &mut conn,
        &profile_id,
        &work_id,
        WorkPatch {
            current_version_id: Some(Some(first.id.clone())),
            ..WorkPatch::default()
        },
    );

    let offer = undo::last(&conn)
        .unwrap()
        .expect("the choice can be undone");
    assert_eq!(offer.action, "undo.work.update");
    take_back(&mut conn, &offer);
    assert_eq!(
        work::get(&conn, &work_id)
            .unwrap()
            .unwrap()
            .current_version_id
            .as_deref(),
        Some(second.id.as_str()),
        "the version current before the choice is current again"
    );
}

/// A profile is saved as one document and taken back as one.
#[test]
fn a_profile_edit_is_taken_back_whole() {
    let (mut conn, profile_id, _) = workspace();
    let before = profile::config_for(&conn, &profile_id).unwrap();

    let mut edited = before.clone();
    let as_seeded = serde_json::to_value(&edited).unwrap();
    edited.work_kinds.retain(|kind| kind.key != "instrumental");
    assert_ne!(
        serde_json::to_value(&edited).unwrap(),
        as_seeded,
        "the edit changes something"
    );

    actions::profile::update_config(&conn, &profile_id, &edited).unwrap();

    let offer = undo::last(&conn)
        .unwrap()
        .expect("the profile edit can be undone");
    assert_eq!(offer.action, "undo.profile.update");
    take_back(&mut conn, &offer);

    assert_eq!(
        serde_json::to_value(profile::config_for(&conn, &profile_id).unwrap()).unwrap(),
        serde_json::to_value(&before).unwrap(),
        "the document came back as it was"
    );
}

/// Undoing the version an editing session minted throws it away — into the
/// trash, like every other undone creation, so a second change of mind is
/// one click.
#[test]
fn a_created_version_is_taken_back_into_the_trash() {
    let (mut conn, profile_id, work_id) = workspace();
    let _key = profile::key_for_id(&conn, &profile_id).unwrap().unwrap();
    let new: kilna_lib::work::version::NewVersion =
        serde_json::from_value(serde_json::json!({ "role": "lyrics", "body": "a first change" }))
            .unwrap();
    let version_id = kilna_lib::actions::version::create(&conn, &work_id, new)
        .unwrap()
        .id;

    let offer = undo::last(&conn)
        .unwrap()
        .expect("the creation can be undone");
    assert_eq!(offer.action, "undo.version.create");
    take_back(&mut conn, &offer);

    assert!(
        version::get(&conn, &version_id).unwrap().is_none(),
        "the version is still there"
    );
    let trashed: i64 = conn
        .query_row(
            "SELECT count(*) FROM deletion WHERE entity = 'version' AND entity_id = ?1",
            [&version_id],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(
        trashed, 1,
        "an undone creation goes to the trash, not into thin air"
    );
}

/// Nothing to undo in a fresh workspace, and saying so is not an error.
#[test]
fn an_empty_log_offers_nothing() {
    let conn = db::open_in_memory().unwrap();
    profile::seed(&conn).unwrap();

    assert!(undo::last(&conn).unwrap().is_none());
}

/// A scene added is taken back into the trash; an edit to one puts back only
/// the fields the edit named — the blocks as a set, and nothing beside them.
#[test]
fn a_scene_is_taken_back_both_ways() {
    let (mut conn, profile_id, _song_id) = workspace();
    let video_id = fixtures::video(&conn, &profile_id, "Harbour lights").id;

    // Added the way the command adds it.
    let scene_id = actions::scene::create(
        &conn,
        kilna_lib::scene::NewScene {
            work_id: video_id.clone(),
            description: Some("a lighthouse at dusk".into()),
            ..kilna_lib::scene::NewScene::default()
        },
    )
    .unwrap()
    .id;

    // Edited the way the command edits it: the blocks as a set, and a section.
    let patch = kilna_lib::scene::ScenePatch {
        section: Some(Some("chorus".into())),
        blocks: Some(
            serde_json::json!({ "still": "a lighthouse, warm light" })
                .as_object()
                .unwrap()
                .clone(),
        ),
        ..kilna_lib::scene::ScenePatch::default()
    };
    actions::scene::update(&conn, &scene_id, patch).unwrap();
    // Something else changes the description meanwhile; the undo must leave it.
    kilna_lib::scene::update(
        &conn,
        &scene_id,
        kilna_lib::scene::ScenePatch {
            description: Some("a gull over the harbour".into()),
            ..kilna_lib::scene::ScenePatch::default()
        },
    )
    .unwrap();
    // That edit was not logged, so the offer is still the logged one.

    let offer = undo::last(&conn)
        .unwrap()
        .expect("an edit to a scene can be undone");
    assert_eq!(offer.action, "undo.scene.update");
    take_back(&mut conn, &offer);
    let back = kilna_lib::scene::get(&conn, &scene_id).unwrap().unwrap();
    assert!(back.section.is_none(), "the section is back to none");
    assert!(
        back.blocks.is_empty(),
        "the blocks are back to the empty set"
    );
    assert_eq!(
        back.description, "a gull over the harbour",
        "a field the edit did not name is left alone"
    );

    // The undo of the update is itself logged; the creation is further back
    // and no longer the last operation, so it is checked on its own board.
    let (mut conn, profile_id, _) = workspace();
    let video_id = fixtures::video(&conn, &profile_id, "Harbour lights").id;
    let scene_id = actions::scene::create(
        &conn,
        kilna_lib::scene::NewScene {
            work_id: video_id,
            ..kilna_lib::scene::NewScene::default()
        },
    )
    .unwrap()
    .id;

    let offer = undo::last(&conn)
        .unwrap()
        .expect("a scene added can be undone");
    assert_eq!(offer.action, "undo.scene.create");
    take_back(&mut conn, &offer);
    assert!(
        kilna_lib::scene::get(&conn, &scene_id).unwrap().is_none(),
        "the scene is still on the board"
    );
    let trashed = kilna_lib::trash::list(&conn, &profile_id).unwrap();
    assert!(
        trashed
            .iter()
            .any(|entry| entry.entity_id == scene_id && entry.label == "Scene 1"),
        "into the trash, not destroyed: {trashed:?}"
    );
}

/// Timing a board is one gesture, and taking it back is one too: every scene
/// goes back to the span it held, including the ones that held none. Undoing
/// it scene by scene would leave the board in as many half-timed states as it
/// has scenes.
#[test]
fn a_timed_board_is_taken_back_whole() {
    let (mut conn, profile_id, _song_id) = workspace();
    let _key = profile::key_for_id(&conn, &profile_id).unwrap().unwrap();
    let video_id = work::create(
        &conn,
        &profile_id,
        NewWork {
            kind: "video".into(),
            title: "Harbour lights".into(),
            meta: Some(
                serde_json::json!({ "duration": 90 })
                    .as_object()
                    .unwrap()
                    .clone(),
            ),
            ..NewWork::default()
        },
    )
    .unwrap()
    .id;

    let mut scene_ids = Vec::new();
    for _ in 0..3 {
        scene_ids.push(
            kilna_lib::scene::create(
                &conn,
                &profile_id,
                kilna_lib::scene::NewScene {
                    work_id: video_id.clone(),
                    ..kilna_lib::scene::NewScene::default()
                },
            )
            .unwrap()
            .id,
        );
    }

    // One scene was timed by hand before the board was: its span is what the
    // undo has to put back, not "nothing".
    kilna_lib::scene::update(
        &conn,
        &scene_ids[1],
        kilna_lib::scene::ScenePatch {
            starts_at: Some(Some(12.0)),
            ends_at: Some(Some(15.0)),
            ..kilna_lib::scene::ScenePatch::default()
        },
    )
    .unwrap();

    kilna_lib::actions::scene::time(&conn, &video_id).unwrap();

    let timed = kilna_lib::scene::for_work(&conn, &video_id).unwrap();
    assert_eq!(timed[0].starts_at, Some(0.0));
    assert_eq!(timed[2].ends_at, Some(90.0));

    let offer = undo::last(&conn)
        .unwrap()
        .expect("a timed board can be undone");
    assert_eq!(offer.action, "undo.scene.time");
    take_back(&mut conn, &offer);

    let back = kilna_lib::scene::for_work(&conn, &video_id).unwrap();
    assert!(
        back[0].starts_at.is_none() && back[0].ends_at.is_none(),
        "a scene that held no span holds none again"
    );
    assert_eq!(
        (back[1].starts_at, back[1].ends_at),
        (Some(12.0), Some(15.0)),
        "the span set by hand comes back as it was, not as nothing"
    );
    assert!(
        back[2].starts_at.is_none(),
        "the last scene is back to untimed too"
    );
}

/// Renumbering is one gesture over the whole board, so taking it back is one
/// too: the order the board stood in comes back, not four scenes out of five
/// while the fifth keeps the number it was given.
///
/// The board here is deliberately crooked to start with — a number typed by
/// hand, a twin — because that is the state `position` allows and the state a
/// renumbering is called on. The undo has to restore THAT, not a tidy
/// 1,2,3,4: putting the board somewhere it never was is not taking anything
/// back.
#[test]
fn a_renumbered_board_goes_back_to_the_order_it_stood_in() {
    let (mut conn, profile_id, _song_id) = workspace();
    let _key = profile::key_for_id(&conn, &profile_id).unwrap().unwrap();
    let video_id = fixtures::video(&conn, &profile_id, "Harbour lights").id;

    let mut scene_ids = Vec::new();
    for position in [3, 1, 3, 9] {
        scene_ids.push(
            kilna_lib::scene::create(
                &conn,
                &profile_id,
                kilna_lib::scene::NewScene {
                    work_id: video_id.clone(),
                    position: Some(position),
                    ..kilna_lib::scene::NewScene::default()
                },
            )
            .unwrap()
            .id,
        );
    }

    let numbers = |conn: &rusqlite::Connection| -> Vec<(String, i64)> {
        kilna_lib::scene::for_work(conn, &video_id)
            .unwrap()
            .into_iter()
            .map(|scene| (scene.id, scene.position))
            .collect()
    };
    let crooked = numbers(&conn);

    // The numbers travel in the operation's `before`, so the undo can put
    // back a crooked board rather than a tidy one it has never been.
    let wanted: Vec<String> = crooked.iter().rev().map(|(id, _)| id.clone()).collect();
    kilna_lib::actions::scene::renumber(&conn, &video_id, &wanted).unwrap();

    assert_eq!(
        numbers(&conn)
            .into_iter()
            .map(|(_, position)| position)
            .collect::<Vec<_>>(),
        [1, 2, 3, 4],
        "the board came out straight, whatever it held before"
    );
    assert_eq!(
        numbers(&conn)
            .into_iter()
            .map(|(id, _)| id)
            .collect::<Vec<_>>(),
        wanted,
        "and in the order asked for"
    );

    let offer = undo::last(&conn)
        .unwrap()
        .expect("a renumbered board can be undone");
    assert_eq!(offer.action, "undo.scene.renumber");
    take_back(&mut conn, &offer);

    assert_eq!(
        numbers(&conn),
        crooked,
        "every scene holds the number it held, crooked board and all"
    );
}

/// Framing a board makes every scene in one gesture, so taking it back
/// leaves none of them behind — in the trash, not destroyed, for the person
/// who undid by mistake.
#[test]
fn a_framed_board_is_taken_back_whole() {
    let (mut conn, profile_id, song_id) = workspace();
    let _key = profile::key_for_id(&conn, &profile_id).unwrap().unwrap();
    kilna_lib::work::version::create(
        &conn,
        &song_id,
        kilna_lib::work::version::NewVersion {
            role: "lyrics".into(),
            body: "[Intro]\n\n[Verse 1]\na line\n\n[Chorus]\nthe hook\n".into(),
            label: None,
            meta: None,
            make_current: true,
            parent_version_id: None,
        },
    )
    .unwrap();

    let video_id = fixtures::video(&conn, &profile_id, "Harbour lights").id;
    kilna_lib::link::create(
        &conn,
        &profile_id,
        kilna_lib::link::NewLink {
            work_id: video_id.clone(),
            source_id: song_id.clone(),
            role: None,
            source_version_id: None,
        },
    )
    .unwrap();

    let framed = kilna_lib::actions::scene::frame(&conn, &video_id, "lyrics").unwrap();
    let ids: Vec<String> = framed.iter().map(|scene| scene.id.clone()).collect();
    assert_eq!(framed.len(), 3);

    let offer = undo::last(&conn)
        .unwrap()
        .expect("a framed board can be undone");
    assert_eq!(offer.action, "undo.scene.frame");
    take_back(&mut conn, &offer);

    assert_eq!(
        kilna_lib::scene::count(&conn, &video_id).unwrap(),
        0,
        "the whole frame goes, not all but one"
    );
    let trashed = kilna_lib::trash::list(&conn, &profile_id).unwrap();
    assert_eq!(
        ids.iter()
            .filter(|id| trashed.iter().any(|entry| &&entry.entity_id == id))
            .count(),
        3,
        "into the trash, not destroyed: {trashed:?}"
    );
}

/// Choosing a frame is taken back to the frame that was chosen before, not to
/// an undecided scene. A board where the undo forgot the previous verdict
/// would quietly lose a decision the person had already made.
#[test]
fn undoing_a_chosen_frame_puts_the_previous_one_back() {
    let (mut conn, profile_id, _song_id) = workspace();
    let media = tempfile::tempdir().unwrap();
    let source = tempfile::tempdir().unwrap();

    let video_id = fixtures::video(&conn, &profile_id, "Harbour lights").id;
    let scene_id = kilna_lib::scene::create(
        &conn,
        &profile_id,
        kilna_lib::scene::NewScene {
            work_id: video_id,
            ..kilna_lib::scene::NewScene::default()
        },
    )
    .unwrap()
    .id;

    let mut frames = Vec::new();
    for name in ["still-v1.png", "still-v2.png"] {
        let file = source.path().join(name);
        std::fs::write(&file, b"not really a png").unwrap();
        frames.push(
            kilna_lib::scene_frame::attach(
                &conn,
                media.path(),
                &scene_id,
                kilna_lib::scene_frame::FRAME,
                &file,
            )
            .unwrap(),
        );
    }

    // The first verdict, unlogged: it is the state the logged one replaces.
    kilna_lib::scene_frame::select(&conn, &frames[0].id).unwrap();

    // The second, recorded the way the command records it.
    actions::scene::select_frame(&conn, &frames[1].id).unwrap();

    let offer = undo::last(&conn)
        .unwrap()
        .expect("choosing a frame can be undone");
    assert_eq!(offer.action, "undo.scene.selectFrame");
    take_back(&mut conn, &offer);

    let chosen: Vec<_> = kilna_lib::scene_frame::for_scene(&conn, &scene_id)
        .unwrap()
        .into_iter()
        .filter(|one| one.is_selected)
        .collect();
    assert_eq!(chosen.len(), 1, "still exactly one verdict");
    assert_eq!(
        chosen[0].id, frames[0].id,
        "and it is the frame that was chosen before, not none"
    );
}

/// Describing a style is taken back to what stood there — blank included.
///
/// The interesting half is the status. A brick is a draft until it carries a
/// description; describing it makes it ready. An undo that put the old text
/// back and left the brick `ready` would leave a draft masquerading as a
/// finished part, and the constructor would offer it. So both travel in the
/// `before`, and both come back.
#[test]
fn undoing_a_description_puts_back_the_draft_it_was() {
    use kilna_lib::style_brick::{self, DRAFT, NewStyleBrick, READY, StyleBrickPatch};

    let (mut conn, profile_id, _work_id) = workspace();
    let key = profile::key_for_id(&conn, &profile_id).unwrap().unwrap();

    let brick = style_brick::create(
        &conn,
        &profile_id,
        NewStyleBrick {
            type_key: "image-style".into(),
            name: "Cold north".into(),
            description: None,
            hint: None,
        },
    )
    .unwrap();
    assert_eq!(brick.status, DRAFT);

    // Exactly what the command records, including the `before` it carries.
    let at = kilna_lib::time::now();
    let logged = operation::Intent::new("style.describe")
        .in_profile(&profile_id)
        .param("profile", key)
        .param("id", brick.id.clone())
        .param(
            "before",
            serde_json::to_value(StyleBrickPatch {
                description: Some(brick.description.clone()),
                status: Some(brick.status.clone()),
                ..Default::default()
            })
            .unwrap(),
        )
        .param("at", at.clone());

    let transaction = conn.transaction().unwrap();
    style_brick::describe(&transaction, &brick.id, "Grainy monochrome film.").unwrap();
    operation::record(&transaction, logged).unwrap();
    transaction.commit().unwrap();

    let described = style_brick::get(&conn, &brick.id).unwrap().unwrap();
    assert_eq!(described.status, READY);

    let offer = undo::last(&conn)
        .unwrap()
        .expect("describing a style can be undone");
    assert_eq!(offer.action, "undo.style.describe");
    take_back(&mut conn, &offer);

    let back = style_brick::get(&conn, &brick.id).unwrap().unwrap();
    assert_eq!(back.description, None, "the text did not come back");
    assert_eq!(
        back.status, DRAFT,
        "a brick whose description was undone is a draft again, not a ready one with no text"
    );
}

/// Promoting a note is taken back as the gesture it was: the work goes, the
/// note returns. Restoring only the note would leave its text in two places.
#[test]
fn a_promotion_is_taken_back_whole() {
    use kilna_lib::note::{self, NewNote, Promotion};

    let (mut conn, profile_id, _) = workspace();
    let _key = profile::key_for_id(&conn, &profile_id).unwrap().unwrap();
    let idea = note::create(
        &conn,
        &profile_id,
        NewNote {
            body: "a comma in the rock".into(),
            kind: None,
            title: None,
            work_id: None,
            tags: vec![],
            ..Default::default()
        },
    )
    .unwrap();

    let promotion = Promotion {
        kind: "song".into(),
        title: "Graphite".into(),
    };
    let promoted = kilna_lib::actions::note::promote(&conn, &idea.id, promotion).unwrap();

    let offer = undo::last(&conn)
        .unwrap()
        .expect("a promotion can be undone");
    assert_eq!(offer.action, "undo.note.promote");
    take_back(&mut conn, &offer);

    assert!(
        work::get(&conn, &promoted.work_id).unwrap().is_none(),
        "the work the promotion made is still there"
    );
    assert_eq!(
        note::get(&conn, &idea.id).unwrap().unwrap().body,
        "a comma in the rock",
        "the note did not come back"
    );
}

#[test]
fn a_spent_idea_goes_back_to_fresh_when_its_promotion_is_taken_back() {
    use kilna_lib::note::{self, NewNote, NoteState, Promotion};

    let (mut conn, profile_id, _) = workspace();
    let idea = note::create(
        &conn,
        &profile_id,
        NewNote {
            body: "a lighthouse keeps the hours".into(),
            kind: Some("idea".into()),
            ..Default::default()
        },
    )
    .unwrap();

    let promoted = kilna_lib::actions::note::promote(
        &conn,
        &idea.id,
        Promotion {
            kind: "song".into(),
            title: "Lighthouse".into(),
        },
    )
    .unwrap();
    let spent = note::get(&conn, &idea.id).unwrap().unwrap();
    assert_eq!(spent.state, NoteState::Used, "an idea is spent, not moved");
    assert_eq!(spent.work_id.as_deref(), Some(promoted.work_id.as_str()));

    let offer = undo::last(&conn)
        .unwrap()
        .expect("the promotion can be undone");
    take_back(&mut conn, &offer);

    assert!(work::get(&conn, &promoted.work_id).unwrap().is_none());
    let back = note::get(&conn, &idea.id).unwrap().unwrap();
    assert_eq!(
        back.state,
        NoteState::Fresh,
        "the idea is there to use again"
    );
    assert_eq!(back.work_id, None);
}

#[test]
fn a_term_and_the_works_named_for_it_are_taken_back() {
    use kilna_lib::actions::register as gestures;
    use kilna_lib::register::{self, NewTerm, TermKind, TermPatch};

    let (mut conn, profile_id, _) = workspace();
    let song = work::create(
        &conn,
        &profile_id,
        work::NewWork {
            kind: "song".into(),
            title: "Harbour lights".into(),
            ..work::NewWork::default()
        },
    )
    .unwrap();
    let term = gestures::create(
        &conn,
        NewTerm {
            word: "a lighthouse nobody keeps".into(),
            kind: Some(TermKind::Image),
            ..NewTerm::default()
        },
    )
    .unwrap();

    gestures::update(
        &conn,
        &term.id,
        TermPatch {
            note: Some(Some("every other song".into())),
            ..TermPatch::default()
        },
    )
    .unwrap();
    let offer = undo::last(&conn).unwrap().expect("an edit can be undone");
    take_back(&mut conn, &offer);
    assert_eq!(register::get(&conn, &term.id).unwrap().unwrap().note, None);

    gestures::link(&conn, &term.id, &song.id).unwrap();
    let offer = undo::last(&conn)
        .unwrap()
        .expect("naming a work can be undone");
    take_back(&mut conn, &offer);
    assert!(register::uses(&conn, &term.id).unwrap().is_empty());

    gestures::link(&conn, &term.id, &song.id).unwrap();
    gestures::unlink(&conn, &term.id, &song.id).unwrap();
    let offer = undo::last(&conn)
        .unwrap()
        .expect("letting go can be undone");
    take_back(&mut conn, &offer);
    assert_eq!(
        register::uses(&conn, &term.id).unwrap().len(),
        1,
        "named again"
    );

    let made = gestures::create(
        &conn,
        NewTerm {
            word: "dust".into(),
            ..NewTerm::default()
        },
    )
    .unwrap();
    let offer = undo::last(&conn)
        .unwrap()
        .expect("a new term can be undone");
    take_back(&mut conn, &offer);
    assert!(
        register::get(&conn, &made.id).unwrap().is_none(),
        "undoing a term's making puts it in the trash"
    );
}

/// A reply written onto a comment is taken back to what stood there.
#[test]
fn a_reply_is_taken_back() {
    use kilna_lib::comment::{self, NewComment};

    let (mut conn, profile_id, _) = workspace();
    let kept = comment::create_minted(
        &conn,
        &profile_id,
        NewComment {
            channel: "main".into(),
            body: "what is the bridge about?".into(),
            reply: Some("first thought".into()),
            ..NewComment::default()
        },
        Minted::fresh(),
    )
    .unwrap();

    actions::comment::reply(&conn, &kept.id, "a better answer").unwrap();

    let offer = undo::last(&conn)
        .unwrap()
        .expect("an edit to a comment can be undone");
    assert_eq!(offer.action, "undo.comment.update");
    take_back(&mut conn, &offer);

    assert_eq!(
        comment::get(&conn, &kept.id)
            .unwrap()
            .unwrap()
            .reply
            .as_deref(),
        Some("first thought")
    );
}

/// A card of the canon to write facts on, made the way the window makes it.
fn a_card(conn: &Connection, title: &str) -> String {
    actions::canon::create_card(
        conn,
        kilna_lib::note::NewNote {
            kind: Some("character".into()),
            title: Some(title.into()),
            ..kilna_lib::note::NewNote::default()
        },
    )
    .unwrap()
    .id
}

fn a_fact(conn: &Connection, card: &str, section: &str, body: &str) -> kilna_lib::canon::Fact {
    actions::canon::add_fact(
        conn,
        kilna_lib::canon::NewFact {
            note_id: card.into(),
            section: section.into(),
            body: body.into(),
            ..kilna_lib::canon::NewFact::default()
        },
    )
    .unwrap()
}

#[test]
fn a_fact_added_goes_to_the_trash_and_an_edit_comes_back_to_where_it_stood() {
    use kilna_lib::canon::{self, FactPatch};

    let (mut conn, _, _) = workspace();
    let card = a_card(&conn, "Wren");
    let hair = a_fact(&conn, &card, "looks", "Gradient hair.");
    a_fact(&conn, &card, "looks", "Freckles.");

    let freckled = a_fact(&conn, &card, "looks", "A third.");
    let offer = undo::last(&conn)
        .unwrap()
        .expect("adding a fact can be undone");
    assert_eq!(offer.action, "undo.fact.create");
    take_back(&mut conn, &offer);
    assert!(canon::fact::get(&conn, &freckled.id).unwrap().is_none());

    // Moved to another section and made internal: the undo puts it back in
    // its section, at its place, in its layer - and nothing else.
    actions::canon::update_fact(
        &conn,
        &hair.id,
        FactPatch {
            section: Some("symbols".into()),
            layer: Some(canon::Layer::Internal),
            ..FactPatch::default()
        },
    )
    .unwrap();
    let offer = undo::last(&conn).unwrap().unwrap();
    assert_eq!(offer.action, "undo.fact.update");
    take_back(&mut conn, &offer);
    let back = canon::fact::get(&conn, &hair.id).unwrap().unwrap();
    assert_eq!(back.section, "looks");
    assert_eq!(back.position, 1);
    assert_eq!(back.layer, canon::Layer::Public);

    actions::canon::retire_fact(&conn, &hair.id, "the loose hair stays").unwrap();
    let offer = undo::last(&conn).unwrap().unwrap();
    take_back(&mut conn, &offer);
    let live = canon::fact::get(&conn, &hair.id).unwrap().unwrap();
    assert_eq!(live.status, canon::FactStatus::Canon);
    assert_eq!(live.retired_reason, None);
}

#[test]
fn an_order_of_facts_goes_back_to_the_one_it_replaced() {
    use kilna_lib::canon;

    let (mut conn, _, _) = workspace();
    let card = a_card(&conn, "Wren");
    let a = a_fact(&conn, &card, "looks", "a").id;
    let b = a_fact(&conn, &card, "looks", "b").id;
    let c = a_fact(&conn, &card, "looks", "c").id;

    actions::canon::reorder_facts(&conn, &card, "looks", &[c.clone(), a.clone(), b.clone()])
        .unwrap();
    let offer = undo::last(&conn).unwrap().unwrap();
    assert_eq!(offer.action, "undo.fact.reorder");
    take_back(&mut conn, &offer);

    let order: Vec<String> = canon::fact::in_section(&conn, &card, "looks")
        .unwrap()
        .into_iter()
        .map(|f| f.id)
        .collect();
    assert_eq!(order, [a, b, c]);
}

#[test]
fn a_relation_undrawn_comes_back_as_the_same_row() {
    use kilna_lib::canon::{self, CanonLinkPatch, NewCanonLink};

    let (mut conn, _, _) = workspace();
    let wren = a_card(&conn, "Wren");
    let otto = a_card(&conn, "Otto");
    let drawn = actions::canon::relate(
        &conn,
        NewCanonLink {
            from_id: wren.clone(),
            to_id: otto.clone(),
            label: Some("neighbour".into()),
            ..NewCanonLink::default()
        },
    )
    .unwrap();

    actions::canon::update_relation(
        &conn,
        &drawn.id,
        CanonLinkPatch {
            label: Some(Some("first listener".into())),
            ..CanonLinkPatch::default()
        },
    )
    .unwrap();
    let offer = undo::last(&conn).unwrap().unwrap();
    take_back(&mut conn, &offer);
    assert_eq!(
        canon::link::get(&conn, &drawn.id)
            .unwrap()
            .unwrap()
            .label
            .as_deref(),
        Some("neighbour")
    );

    actions::canon::unrelate(&conn, &drawn.id).unwrap();
    let offer = undo::last(&conn).unwrap().unwrap();
    assert_eq!(offer.action, "undo.canonLink.delete");
    take_back(&mut conn, &offer);
    let back = canon::link::get(&conn, &drawn.id)
        .unwrap()
        .expect("the same row came back");
    assert_eq!(back, drawn);

    let offer = undo::last(&conn).unwrap();
    assert!(offer.is_none(), "an undo was offered for an undo");
    actions::canon::relate(
        &conn,
        NewCanonLink {
            from_id: wren,
            to_id: a_card(&conn, "Pashka"),
            ..NewCanonLink::default()
        },
    )
    .unwrap();
    let offer = undo::last(&conn).unwrap().unwrap();
    assert_eq!(offer.action, "undo.canonLink.create");
    take_back(&mut conn, &offer);
    assert_eq!(canon::link::for_card(&conn, &otto).unwrap().len(), 1);
}
