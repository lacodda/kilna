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

use rusqlite::Connection;

use kilna_lib::minted::Minted;
use kilna_lib::work::version;
use kilna_lib::work::{NewWork, WorkPatch};
use kilna_lib::{db, operation, profile, undo, work};

/// A workspace with one work in it, and the id of that work.
fn workspace() -> (Connection, String, String) {
    let mut conn = db::open_in_memory().unwrap();
    profile::seed(&conn).unwrap();
    let profile_id = profile::active(&conn).unwrap().unwrap().id;
    let key = profile::key_for_id(&conn, &profile_id).unwrap().unwrap();

    let new = NewWork {
        kind: "song".into(),
        title: "Harbour lights".into(),
        ..NewWork::default()
    };
    let minted = Minted::fresh();
    let work_id = minted.id().to_owned();
    let logged = operation::Intent::new("work.create")
        .in_profile(&profile_id)
        .param("profile", key)
        .param("work", serde_json::to_value(&new).unwrap())
        .minted(&minted);

    let transaction = conn.transaction().unwrap();
    work::create_minted(&transaction, &profile_id, new, minted).unwrap();
    operation::record(&transaction, logged).unwrap();
    transaction.commit().unwrap();

    (conn, profile_id, work_id)
}

/// Edit a work the way the command does: patch, `before`, one moment.
fn edit(conn: &mut Connection, profile_id: &str, work_id: &str, patch: WorkPatch) {
    let key = profile::key_for_id(conn, profile_id).unwrap().unwrap();
    let before = work::get(conn, work_id).unwrap().unwrap();
    let before = serde_json::to_value(&before).unwrap();
    let patch_json = serde_json::to_value(&patch).unwrap();
    let inverse =
        kilna_lib::reversal::invert(before.as_object().unwrap(), patch_json.as_object().unwrap());

    let at = kilna_lib::time::now();
    let logged = operation::Intent::new("work.update")
        .in_profile(profile_id)
        .param("profile", key)
        .param("id", work_id.to_owned())
        .param("patch", patch_json)
        .param("before", serde_json::Value::Object(inverse))
        .param("at", at.clone());

    let transaction = conn.transaction().unwrap();
    work::update_at(&transaction, work_id, patch, &at).unwrap();
    operation::record(&transaction, logged).unwrap();
    transaction.commit().unwrap();
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
    undo::undo(&mut conn, &offer.operation_id).unwrap();

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
    undo::undo(&mut conn, &offer.operation_id).unwrap();

    let after = work::get(&conn, &work_id).unwrap().unwrap();
    assert_eq!(after.status, "draft", "the status did not come back");
    assert_eq!(
        after.title, "Winter road",
        "undoing the status change also reverted the rename — the inverse is \
         being built from the whole row instead of from the patch"
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
    undo::undo(&mut conn, &offer.operation_id).unwrap();

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

    let refused = undo::undo(&mut conn, &stale.operation_id);

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
    undo::undo(&mut conn, &offer.operation_id).unwrap();

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
    let (mut conn, profile_id, work_id) = workspace();

    let minted = Minted::fresh();
    let key = profile::key_for_id(&conn, &profile_id).unwrap().unwrap();
    let logged = operation::Intent::new("entity.discard")
        .in_profile(&profile_id)
        .param("profile", key)
        .param("entity", "work")
        .param("entityId", work_id.clone())
        .minted(&minted);
    kilna_lib::trash::discard_minted(
        &mut conn,
        kilna_lib::trash::Entity::Work,
        &work_id,
        minted,
        Some(logged),
    )
    .unwrap();
    assert!(work::get(&conn, &work_id).unwrap().is_none());

    let offer = undo::last(&conn)
        .unwrap()
        .expect("a deletion can be undone");
    undo::undo(&mut conn, &offer.operation_id).unwrap();

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
    let key = profile::key_for_id(&conn, &profile_id).unwrap().unwrap();
    let video_id = work::create(
        &conn,
        &profile_id,
        NewWork {
            kind: "video".into(),
            title: "Harbour lights".into(),
            ..NewWork::default()
        },
    )
    .unwrap()
    .id;

    let new = kilna_lib::link::NewLink {
        work_id: video_id.clone(),
        source_id: song_id.clone(),
        role: None,
        source_version_id: None,
    };
    let minted = Minted::fresh();
    let link_id = minted.id().to_owned();
    let logged = operation::Intent::new("link.create")
        .in_profile(&profile_id)
        .param("profile", key.clone())
        .param("link", serde_json::to_value(&new).unwrap())
        .minted(&minted);
    let transaction = conn.transaction().unwrap();
    kilna_lib::link::create_minted(&transaction, &profile_id, new, minted).unwrap();
    operation::record(&transaction, logged).unwrap();
    transaction.commit().unwrap();

    let offer = undo::last(&conn)
        .unwrap()
        .expect("a link made can be undone");
    assert_eq!(offer.action, "undo.link.create");
    undo::undo(&mut conn, &offer.operation_id).unwrap();
    assert!(
        kilna_lib::link::get(&conn, &link_id).unwrap().is_none(),
        "the link is still there"
    );

    // Made again, then removed the way the command removes it, then undone.
    let minted = Minted::fresh();
    let link_id = minted.id().to_owned();
    let transaction = conn.transaction().unwrap();
    let made = kilna_lib::link::create_minted(
        &transaction,
        &profile_id,
        kilna_lib::link::NewLink {
            work_id: video_id.clone(),
            source_id: song_id.clone(),
            role: None,
            source_version_id: None,
        },
        minted,
    )
    .unwrap();
    transaction.commit().unwrap();
    let logged = operation::Intent::new("link.delete")
        .in_profile(&profile_id)
        .param("profile", key)
        .param("id", link_id.clone())
        .param(
            "before",
            serde_json::json!({
                "work_id": made.work_id, "source_id": made.source_id, "role": made.role,
                "source_version_id": made.source_version_id, "created_at": made.created_at,
            }),
        );
    let transaction = conn.transaction().unwrap();
    kilna_lib::link::delete(&transaction, &link_id).unwrap();
    operation::record(&transaction, logged).unwrap();
    transaction.commit().unwrap();
    assert!(kilna_lib::link::get(&conn, &link_id).unwrap().is_none());

    let offer = undo::last(&conn)
        .unwrap()
        .expect("a link removed can be undone");
    assert_eq!(offer.action, "undo.link.delete");
    undo::undo(&mut conn, &offer.operation_id).unwrap();

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
    const NOT_UNDOABLE: [(&str, &str); 24] = [
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
            "the copy it made would have to be unmade, and the row is the only              thing the log holds — taking it back is `detach_asset`, which              removes the bytes as well",
        ),
        (
            "asset.detach",
            "the bytes are gone with the row; a row put back beside a file that              no longer exists is the broken picture the deletion avoided",
        ),
        (
            "scene.attachFrame",
            "it carries a file into the workspace, as `asset.attach` does, and              taking it back is `detach_scene_frame`, which removes the bytes too",
        ),
        (
            "scene.detachFrame",
            "the picture is gone with the row; putting the row back beside a              file that no longer exists is the broken frame the deletion avoided",
        ),
    ];

    // Kinds handled by the trash rather than by an operation of their own.
    const HANDLED_ELSEWHERE: [&str; 2] = ["trash.purge", "trash.empty"];

    let source = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/commands.rs"),
    )
    .expect("commands.rs is readable");

    let mut kinds: Vec<String> = Vec::new();
    for (at, _) in source.match_indices("Intent::new(\"") {
        let start = at + "Intent::new(\"".len();
        let Some(end) = source[start..].find('"') else {
            continue;
        };
        let kind = source[start..start + end].to_owned();
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
    let (mut conn, profile_id, work_id) = workspace();
    let key = profile::key_for_id(&conn, &profile_id).unwrap().unwrap();
    let version = version::create(
        &mut conn,
        &work_id,
        serde_json::from_value(serde_json::json!({ "role": "lyrics", "body": "as written" }))
            .unwrap(),
    )
    .unwrap();

    let at = kilna_lib::time::now();
    let logged = operation::Intent::new("version.edit")
        .in_profile(&profile_id)
        .param("profile", key)
        .param("id", version.id.clone())
        .param("body", "as rewritten")
        .param("before", "as written")
        .param("at", at.clone());
    let transaction = conn.transaction().unwrap();
    version::update_body_at(&transaction, &version.id, "as rewritten", &at).unwrap();
    operation::record(&transaction, logged).unwrap();
    transaction.commit().unwrap();

    let offer = undo::last(&conn).unwrap().expect("the edit can be undone");
    assert_eq!(offer.action, "undo.version.edit");
    undo::undo(&mut conn, &offer.operation_id).unwrap();

    let back = version::get(&conn, &version.id).unwrap().unwrap();
    assert_eq!(back.body, "as written", "the text did not come back");
    assert_eq!(back.revision, version.revision);
}

/// Undoing the version an editing session minted throws it away — into the
/// trash, like every other undone creation, so a second change of mind is
/// one click.
#[test]
fn a_created_version_is_taken_back_into_the_trash() {
    let (mut conn, profile_id, work_id) = workspace();
    let key = profile::key_for_id(&conn, &profile_id).unwrap().unwrap();
    let new: kilna_lib::work::version::NewVersion =
        serde_json::from_value(serde_json::json!({ "role": "lyrics", "body": "a first change" }))
            .unwrap();
    let minted = Minted::fresh();
    let version_id = minted.id().to_owned();
    let logged = operation::Intent::new("version.create")
        .in_profile(&profile_id)
        .param("profile", key)
        .param("workId", work_id.clone())
        .param("version", serde_json::to_value(&new).unwrap())
        .minted(&minted);
    version::create_minted(&mut conn, &work_id, new, minted, Some(logged)).unwrap();

    let offer = undo::last(&conn)
        .unwrap()
        .expect("the creation can be undone");
    assert_eq!(offer.action, "undo.version.create");
    undo::undo(&mut conn, &offer.operation_id).unwrap();

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
    let key = profile::key_for_id(&conn, &profile_id).unwrap().unwrap();
    let video_id = work::create(
        &conn,
        &profile_id,
        NewWork {
            kind: "video".into(),
            title: "Harbour lights".into(),
            ..NewWork::default()
        },
    )
    .unwrap()
    .id;

    // Added the way the command adds it.
    let new = kilna_lib::scene::NewScene {
        work_id: video_id.clone(),
        description: Some("a lighthouse at dusk".into()),
        ..kilna_lib::scene::NewScene::default()
    };
    let minted = Minted::fresh();
    let scene_id = minted.id().to_owned();
    let logged = operation::Intent::new("scene.create")
        .in_profile(&profile_id)
        .param("profile", key.clone())
        .param("scene", serde_json::to_value(&new).unwrap())
        .minted(&minted);
    let transaction = conn.transaction().unwrap();
    kilna_lib::scene::create_minted(&transaction, &profile_id, new, minted).unwrap();
    operation::record(&transaction, logged).unwrap();
    transaction.commit().unwrap();

    // Edited the way the command edits it: the blocks as a set, and a section.
    let before = kilna_lib::scene::get(&conn, &scene_id).unwrap().unwrap();
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
    let before_json = serde_json::to_value(&before).unwrap();
    let patch_json = serde_json::to_value(&patch).unwrap();
    let inverse = kilna_lib::reversal::invert(
        before_json.as_object().unwrap(),
        patch_json.as_object().unwrap(),
    );
    let at = kilna_lib::time::now();
    let logged = operation::Intent::new("scene.update")
        .in_profile(&profile_id)
        .param("profile", key.clone())
        .param("id", scene_id.clone())
        .param("patch", patch_json.clone())
        .param("before", serde_json::Value::Object(inverse))
        .param("at", at.clone());
    let transaction = conn.transaction().unwrap();
    kilna_lib::scene::update_at(&transaction, &scene_id, patch, &at).unwrap();
    operation::record(&transaction, logged).unwrap();
    transaction.commit().unwrap();
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
    undo::undo(&mut conn, &offer.operation_id).unwrap();
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
    let key = profile::key_for_id(&conn, &profile_id).unwrap().unwrap();
    let video_id = work::create(
        &conn,
        &profile_id,
        NewWork {
            kind: "video".into(),
            title: "Harbour lights".into(),
            ..NewWork::default()
        },
    )
    .unwrap()
    .id;
    let new = kilna_lib::scene::NewScene {
        work_id: video_id,
        ..kilna_lib::scene::NewScene::default()
    };
    let minted = Minted::fresh();
    let scene_id = minted.id().to_owned();
    let logged = operation::Intent::new("scene.create")
        .in_profile(&profile_id)
        .param("profile", key)
        .param("scene", serde_json::to_value(&new).unwrap())
        .minted(&minted);
    let transaction = conn.transaction().unwrap();
    kilna_lib::scene::create_minted(&transaction, &profile_id, new, minted).unwrap();
    operation::record(&transaction, logged).unwrap();
    transaction.commit().unwrap();

    let offer = undo::last(&conn)
        .unwrap()
        .expect("a scene added can be undone");
    assert_eq!(offer.action, "undo.scene.create");
    undo::undo(&mut conn, &offer.operation_id).unwrap();
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
    let key = profile::key_for_id(&conn, &profile_id).unwrap().unwrap();
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

    // Timed the way the command times it.
    let before: Vec<serde_json::Value> = kilna_lib::scene::for_work(&conn, &video_id)
        .unwrap()
        .into_iter()
        .map(|scene| {
            serde_json::json!({
                "id": scene.id,
                "startsAt": scene.starts_at,
                "endsAt": scene.ends_at,
            })
        })
        .collect();
    let at = kilna_lib::time::now();
    let logged = operation::Intent::new("scene.time")
        .in_profile(&profile_id)
        .param("profile", key.clone())
        .param("workId", video_id.clone())
        .param("before", serde_json::to_value(&before).unwrap())
        .param("at", at.clone());
    kilna_lib::scene::time_board_at(&mut conn, &video_id, &at, Some(logged)).unwrap();

    let timed = kilna_lib::scene::for_work(&conn, &video_id).unwrap();
    assert_eq!(timed[0].starts_at, Some(0.0));
    assert_eq!(timed[2].ends_at, Some(90.0));

    let offer = undo::last(&conn)
        .unwrap()
        .expect("a timed board can be undone");
    assert_eq!(offer.action, "undo.scene.time");
    undo::undo(&mut conn, &offer.operation_id).unwrap();

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
    let key = profile::key_for_id(&conn, &profile_id).unwrap().unwrap();
    let video_id = work::create(
        &conn,
        &profile_id,
        NewWork {
            kind: "video".into(),
            title: "Harbour lights".into(),
            ..NewWork::default()
        },
    )
    .unwrap()
    .id;

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

    // Renumbered the way the command renumbers it: the NUMBERS travel in
    // `before`, so the undo can put back a crooked board rather than a tidy
    // one the board has never been.
    let places: Vec<serde_json::Value> = crooked
        .iter()
        .map(|(id, position)| serde_json::json!({ "id": id, "position": position }))
        .collect();
    let wanted: Vec<String> = crooked.iter().rev().map(|(id, _)| id.clone()).collect();
    let at = kilna_lib::time::now();
    let logged = operation::Intent::new("scene.renumber")
        .in_profile(&profile_id)
        .param("profile", key.clone())
        .param("workId", video_id.clone())
        .param("ids", serde_json::to_value(&wanted).unwrap())
        .param("before", serde_json::to_value(&places).unwrap())
        .param("at", at.clone());
    kilna_lib::scene::renumber(&mut conn, &video_id, &wanted, &at, Some(logged)).unwrap();

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
    undo::undo(&mut conn, &offer.operation_id).unwrap();

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
    let key = profile::key_for_id(&conn, &profile_id).unwrap().unwrap();
    kilna_lib::work::version::create(
        &mut conn,
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

    let video_id = work::create(
        &conn,
        &profile_id,
        NewWork {
            kind: "video".into(),
            title: "Harbour lights".into(),
            ..NewWork::default()
        },
    )
    .unwrap()
    .id;
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

    // Framed the way the command frames it.
    let parts = kilna_lib::scene::parts_of_source(&conn, &video_id, "lyrics").unwrap();
    let minted: Vec<Minted> = (0..parts).map(|_| Minted::fresh()).collect();
    let ids: Vec<String> = minted.iter().map(|one| one.id().to_owned()).collect();
    let at = kilna_lib::time::now();
    let logged = operation::Intent::new("scene.frame")
        .in_profile(&profile_id)
        .param("profile", key.clone())
        .param("workId", video_id.clone())
        .param("role", "lyrics".to_owned())
        .param("ids", serde_json::to_value(&ids).unwrap())
        .param("at", at.clone());
    let framed =
        kilna_lib::scene::frame_from_text(&mut conn, &video_id, "lyrics", &minted, Some(logged))
            .unwrap();
    assert_eq!(framed.len(), 3);

    let offer = undo::last(&conn)
        .unwrap()
        .expect("a framed board can be undone");
    assert_eq!(offer.action, "undo.scene.frame");
    undo::undo(&mut conn, &offer.operation_id).unwrap();

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
    let key = profile::key_for_id(&conn, &profile_id).unwrap().unwrap();
    let media = tempfile::tempdir().unwrap();
    let source = tempfile::tempdir().unwrap();

    let video_id = work::create(
        &conn,
        &profile_id,
        NewWork {
            kind: "video".into(),
            title: "Harbour lights".into(),
            ..NewWork::default()
        },
    )
    .unwrap()
    .id;
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
    let before = kilna_lib::scene_frame::for_scene(&conn, &scene_id)
        .unwrap()
        .into_iter()
        .find(|one| one.is_selected)
        .map(|one| one.id);
    let logged = operation::Intent::new("scene.selectFrame")
        .in_profile(&profile_id)
        .param("profile", key)
        .param("id", frames[1].id.clone())
        .param("sceneId", scene_id.clone())
        .param("before", serde_json::to_value(&before).unwrap());
    let transaction = conn.transaction().unwrap();
    kilna_lib::scene_frame::select(&transaction, &frames[1].id).unwrap();
    operation::record(&transaction, logged).unwrap();
    transaction.commit().unwrap();

    let offer = undo::last(&conn)
        .unwrap()
        .expect("choosing a frame can be undone");
    assert_eq!(offer.action, "undo.scene.selectFrame");
    undo::undo(&mut conn, &offer.operation_id).unwrap();

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
