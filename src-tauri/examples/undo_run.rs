//! Edits a work in a real workspace and takes it back, printing what happened.
//!
//!   cargo run --example undo_run -- <path-to-kilna.db>
//!
//! Exists to try undo against a real catalogue rather than against a fixture:
//! a rename of a work that has versions, scores and releases hanging off it,
//! in a database that has been through every migration since the beginning.
//!
//! Run it on a **copy**. It changes the workspace and puts it back, but that
//! is a promise this program makes, not one the file system enforces.

use std::path::PathBuf;

use kilna_lib::work::WorkPatch;
use kilna_lib::{db, operation, profile, reversal, undo, work};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path: PathBuf = std::env::args()
        .nth(1)
        .ok_or("usage: undo_run <path-to-kilna.db>")?
        .into();

    let mut conn = db::open(&path)?;
    let profile_id = profile::active(&conn)?.ok_or("no active profile")?.id;
    let key = profile::key_for_id(&conn, &profile_id)?.ok_or("the profile has no key")?;

    let subject = work::list(&conn, &profile_id, &work::WorkFilter::default())?
        .into_iter()
        .next()
        .ok_or("the workspace has no works")?;
    let was = subject.title.clone();
    println!("before: {was}");

    // The edit, written exactly as the command writes it.
    let patch = WorkPatch {
        title: Some(format!("{was} (renamed by a test run)")),
        ..WorkPatch::default()
    };
    let before = serde_json::to_value(&subject)?;
    let patch_json = serde_json::to_value(&patch)?;
    let inverse = reversal::invert(
        before.as_object().ok_or("a work is an object")?,
        patch_json.as_object().ok_or("a patch is an object")?,
    );
    let at = kilna_lib::time::now();
    let logged = operation::Intent::new("work.update")
        .in_profile(&profile_id)
        .param("profile", key)
        .param("id", subject.id.clone())
        .param("patch", patch_json)
        .param("before", serde_json::Value::Object(inverse))
        .param("at", at.clone());

    let transaction = conn.transaction()?;
    work::update_at(&transaction, &subject.id, patch, &at)?;
    operation::record(&transaction, logged)?;
    transaction.commit()?;

    let edited = work::get(&conn, &subject.id)?.ok_or("the work vanished")?;
    println!("after edit: {}", edited.title);

    let offer = undo::last(&conn)?.ok_or("nothing was offered to undo")?;
    println!("offered: {}", offer.action);
    undo::undo(&mut conn, &offer.operation_id)?;

    let back = work::get(&conn, &subject.id)?.ok_or("the work vanished")?;
    println!("after undo: {}", back.title);

    if back.title == was {
        println!("\nthe title came back");
    } else {
        return Err(format!("the title did not come back: `{}`", back.title).into());
    }

    // And the log kept both halves: the edit and the taking back of it.
    let recent = operation::latest(&conn, 2)?;
    println!(
        "log tail: {}",
        recent
            .iter()
            .map(|entry| entry.kind.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    );

    Ok(())
}
