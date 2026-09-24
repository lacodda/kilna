//! Taking back the last thing you did.
//!
//! An operation says what was asked for; undoing it means asking for the
//! opposite. That is only possible because the log carries what a field held
//! *before* the change as well as after — see ADR 0014 and the `before` param
//! on every editing operation. Without it an edit could only be undone by
//! replaying the whole history, which on a real catalogue costs seconds per
//! keystroke.
//!
//! Two rules shape the rest:
//!
//! * **The stack lives for the session.** Closing the application empties it.
//!   Undoing arbitrarily far back would need an answer to "what about the work
//!   done on top of this since", and the answer is never simple; a promise that
//!   holds for the last thing you did is one a person can rely on. Deletion is
//!   safe regardless — it goes to the trash, which forgets nothing.
//! * **An undo is itself an operation.** It is recorded like any other change,
//!   so the log stays a complete account of how the database got here. What it
//!   is not is undoable in turn: pressing undo twice takes back the two things
//!   before it, not its own effect.

use rusqlite::Connection;
use serde_json::{Map, Value};

use crate::error::{Error, Result};
use crate::operation::{self, Intent, Operation};

/// What undoing the last operation would take back, in words a person reads.
///
/// Built from the operation's own params rather than from the journal: the
/// journal entry is a sentence about what happened, and this has to name what
/// *unhappening* it would mean. They usually agree; when they do not, the
/// journal is the one that can be reworded.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Undoable {
    /// The operation that would be taken back.
    pub operation_id: String,
    /// The i18n key naming it, e.g. `undo.work.update`.
    pub action: String,
    /// Values the sentence interpolates — usually the title of the thing.
    pub params: Map<String, Value>,
}

/// The last operation that can be taken back, if there is one.
///
/// Reads the log rather than a stack in memory, so a second window — or a
/// command run while the panel was open — cannot leave the offer pointing at
/// something that is no longer last.
pub fn last(conn: &Connection) -> Result<Option<Undoable>> {
    let Some(entry) = operation::latest(conn, 1)?.into_iter().next() else {
        return Ok(None);
    };

    // Only the newest operation is ever offered. Looking further back for
    // something reversible would offer to take back an edit with later work
    // sitting on top of it, which is the question the session-deep stack exists
    // to avoid asking — see the module note.
    //
    // An undo is not itself undoable either: offering that would be a redo
    // wearing the same button, and pressing undo twice would oscillate instead
    // of walking back.
    if entry.kind.starts_with(UNDO_PREFIX) || !reversible(&entry.kind) {
        return Ok(None);
    }

    Ok(Some(Undoable {
        operation_id: entry.id.clone(),
        action: format!("undo.{}", entry.kind),
        params: describing(&entry),
    }))
}

/// Operations recorded by an undo carry this prefix, so they can be told apart
/// from the ones a person asked for directly.
pub const UNDO_PREFIX: &str = "undo.";

/// Whether this kind of operation can be taken back at all.
///
/// Said out loud, and tested, rather than left to whatever the match below
/// happens to cover: a person pressing undo deserves to know the answer before
/// they press, and the offer is only shown for kinds named here.
///
/// Creating a style brick is taken back like every other creation - into the
/// trash - since v0.76.1, when the trash learned to hold bricks. Before that it
/// was the one creation undone by deleting the row outright.
pub fn reversible(kind: &str) -> bool {
    matches!(
        kind,
        "work.create"
            | "work.clone"
            | "work.update"
            | "work.pinTier"
            | "note.create"
            | "note.update"
            | "note.promote"
            | "comment.create"
            | "comment.update"
            | "style.update"
            | "profile.update"
            | "version.create"
            | "version.edit"
            | "collection.create"
            | "collection.update"
            | "release.create"
            | "release.update"
            | "release.markReleased"
            | "entity.discard"
            | "link.create"
            | "link.delete"
            | "style.create"
            | "style.describe"
            | "scene.create"
            | "scene.update"
            | "scene.time"
            | "scene.renumber"
            | "scene.frame"
            | "scene.attachNote"
            | "scene.detachNote"
            | "scene.selectFrame"
            | "scene.clearFrame"
            | "scene.reorderFrames"
            | "cut.create"
            | "cut.update"
            | "cut.reorder"
    )
}

/// Values the sentence about this operation interpolates.
fn describing(entry: &Operation) -> Map<String, Value> {
    let mut params = Map::new();
    for key in ["title", "entity"] {
        if let Some(value) = entry.params.get(key) {
            params.insert(key.to_owned(), value.clone());
        }
    }
    params
}

/// Take back the operation named, and record having done so.
///
/// Refuses rather than guesses when the operation is not the last one, or is
/// not reversible: an undo that silently takes back something else is worse
/// than one that does nothing.
pub fn undo(conn: &mut Connection, operation_id: &str) -> Result<Undoable> {
    let offer = last(conn)?.ok_or_else(|| Error::Other("there is nothing to undo".into()))?;
    if offer.operation_id != operation_id {
        return Err(Error::Other(
            "something else has happened since; there is nothing to undo here".into(),
        ));
    }

    let entry = operation::by_id(conn, operation_id)?
        .ok_or_else(|| Error::not_found("operation", operation_id))?;

    let logged =
        Intent::new(format!("{UNDO_PREFIX}{}", entry.kind)).param("operation", entry.id.clone());
    let logged = match entry.profile_id.as_deref() {
        Some(profile_id) => logged.in_profile(profile_id),
        None => logged,
    };

    reverse(conn, &entry, logged)?;
    Ok(offer)
}

/// Apply the opposite of one operation.
///
/// The undo is stamped with a moment of its own rather than the one it takes
/// back: it is a thing that happens now, and a row claiming it was last touched
/// before the edit it reverses would be lying about its own history.
fn reverse(conn: &mut Connection, entry: &Operation, logged: Intent) -> Result<()> {
    let params = &entry.params;
    let at = crate::time::now();

    match entry.kind.as_str() {
        // An edit is put back by applying what its fields held before. Only
        // those fields — see `crate::reversal` for why that matters.
        "work.update" => {
            let id = required(params, "id")?;
            let patch: crate::work::WorkPatch = from_params(params, "before")?;
            edit(conn, logged, &at, |tx| {
                crate::work::update_at(tx, &id, patch, &at).map(|_| ())
            })?;
        }
        "note.update" => {
            let id = required(params, "id")?;
            let patch: crate::note::NotePatch = from_params(params, "before")?;
            edit(conn, logged, &at, |tx| {
                crate::note::update_at(tx, &id, patch, &at).map(|_| ())
            })?;
        }
        "comment.update" => {
            let id = required(params, "id")?;
            let patch: crate::comment::CommentPatch = from_params(params, "before")?;
            edit(conn, logged, &at, |tx| {
                crate::comment::update_at(tx, &id, patch, &at).map(|_| ())
            })?;
        }
        // The written description goes back to what stood there, blank
        // included: the brick was a draft before it was described, and the
        // `before` carries both the text and the status it had.
        "style.update" | "style.describe" => {
            let id = required(params, "id")?;
            let patch: crate::style_brick::StyleBrickPatch = from_params(params, "before")?;
            edit(conn, logged, &at, |tx| {
                crate::style_brick::update_at(tx, &id, patch, &at).map(|_| ())
            })?;
        }
        // The whole document goes back to what it said: a profile edit is
        // saved as one document, so it is taken back as one.
        "profile.update" => {
            let id = required(params, "id")?;
            let before: crate::profile::config::ProfileConfig = from_params(params, "before")?;
            edit(conn, logged, &at, |tx| {
                crate::profile::update_config_at(tx, &id, &before, &at).map(|_| ())
            })?;
        }
        "scene.update" => {
            let id = required(params, "id")?;
            let patch: crate::scene::ScenePatch = from_params(params, "before")?;
            edit(conn, logged, &at, |tx| {
                crate::scene::update_at(tx, &id, patch, &at).map(|_| ())
            })?;
        }
        "cut.update" => {
            let id = required(params, "id")?;
            let patch: crate::cut::CutPatch = from_params(params, "before")?;
            edit(conn, logged, &at, |tx| {
                crate::cut::update(tx, &id, patch).map(|_| ())
            })?;
        }
        // The splice goes back to the order it held. The list travelled in
        // the operation, so this is the same call with the earlier order.
        "cut.reorder" => {
            let work_id = required(params, "workId")?;
            let ids: Vec<String> = from_params(params, "before")?;
            edit(conn, logged, &at, |tx| {
                crate::cut::reorder(tx, &work_id, &ids).map(|_| ())
            })?;
        }
        // Saying a scene is about someone is taken back by saying it is not,
        // and the other way round. There is nothing to keep: the row holds a
        // scene, a note and the moment, and the moment is the undo's own.
        "scene.attachNote" => {
            let scene_id = required(params, "sceneId")?;
            let note_id = required(params, "noteId")?;
            edit(conn, logged, &at, |tx| {
                crate::scene_note::detach(tx, &scene_id, &note_id)
            })?;
        }
        "scene.detachNote" => {
            let scene_id = required(params, "sceneId")?;
            let note_id = required(params, "noteId")?;
            edit(conn, logged, &at, |tx| {
                crate::scene_note::attach(tx, &scene_id, &note_id).map(|_| ())
            })?;
        }

        // Choosing which frame the video is cut from is taken back to the
        // frame it was cut from before — not to "undecided", which is a
        // different state and not the one the person left.
        "scene.selectFrame" | "scene.clearFrame" => {
            let scene_id = required(params, "sceneId")?;
            let before: Option<String> = from_params(params, "before").unwrap_or(None);
            // Absent on anything logged before 0022, when a still was all a
            // scene could hold.
            let kind = params
                .get("kind")
                .and_then(|value| value.as_str())
                .unwrap_or(crate::scene_frame::FRAME)
                .to_owned();
            edit(conn, logged, &at, |tx| match before.as_deref() {
                Some(id) => crate::scene_frame::select(tx, id).map(|_| ()),
                None => crate::scene_frame::clear_selection(tx, &scene_id, &kind),
            })?;
        }

        // The order is put back as a whole list, the way it was rewritten.
        "scene.reorderFrames" => {
            let scene_id = required(params, "sceneId")?;
            let before: Vec<String> = from_params(params, "before")?;
            let kind = params
                .get("kind")
                .and_then(|value| value.as_str())
                .unwrap_or(crate::scene_frame::FRAME)
                .to_owned();
            edit(conn, logged, &at, |tx| {
                crate::scene_frame::reorder(tx, &scene_id, &kind, &before).map(|_| ())
            })?;
        }

        // A framed board goes to the trash whole: the frame made every scene
        // in one gesture, and taking it back leaves none of them behind. They
        // are in the trash, not destroyed, so a person who undid by mistake
        // has them back.
        "scene.frame" => {
            let ids: Vec<String> = from_params(params, "ids")?;
            let minted: Vec<crate::minted::Minted> =
                ids.iter().map(|_| crate::minted::Minted::fresh()).collect();
            crate::trash::discard_batch(
                conn,
                crate::trash::Entity::Scene,
                &ids,
                &minted,
                Some(logged.param("at", at.clone())),
            )?;
        }

        // A renumbered board goes back to the numbers it held, one row each.
        //
        // The numbers travel, not just the order, because the board a
        // renumbering is called on is often crooked — that is what it is FOR
        // — and putting it back as a tidy 1..N would leave it somewhere it
        // has never been. An undo owes the person the board they had.
        "scene.renumber" => {
            #[derive(serde::Deserialize)]
            struct Place {
                id: String,
                position: i64,
            }
            let before: Vec<Place> = from_params(params, "before")?;
            let places: Vec<(String, i64)> = before
                .into_iter()
                .map(|place| (place.id, place.position))
                .collect();
            edit(conn, logged, &at, |tx| {
                crate::scene::restore_numbers_in(tx, &places, &at)
            })?;
        }

        // A timed board goes back span by span, in one change: the timing was
        // one gesture, and so is taking it back. `before` carries the spans
        // the scenes held, including the ones that held none.
        "scene.time" => {
            #[derive(serde::Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct Span {
                id: String,
                starts_at: Option<f64>,
                ends_at: Option<f64>,
            }
            let before: Vec<Span> = from_params(params, "before")?;
            let spans: Vec<(String, Option<f64>, Option<f64>)> = before
                .into_iter()
                .map(|span| (span.id, span.starts_at, span.ends_at))
                .collect();
            edit(conn, logged, &at, |tx| {
                crate::scene::restore_spans_in(tx, &spans, &at)
            })?;
        }
        "release.update" => {
            let id = required(params, "id")?;
            let patch: crate::release::ReleasePatch = from_params(params, "before")?;
            edit(conn, logged, &at, |tx| {
                crate::release::update_at(tx, &id, patch, &at).map(|_| ())
            })?;
        }
        "collection.update" => {
            let id = required(params, "id")?;
            let patch: crate::collection::CollectionPatch = from_params(params, "before")?;
            edit(conn, logged, &at, |tx| {
                crate::collection::update_at(tx, &id, patch, &at).map(|_| ())
            })?;
        }
        // A body goes back to what it was, whole. Refused the same way the
        // edit would be if the version has been scored since: the score read
        // this text, and an undo may no more rewrite it than a keystroke may.
        "version.edit" => {
            let id = required(params, "id")?;
            let before = required(params, "before")?;
            edit(conn, logged, &at, |tx| {
                crate::work::version::update_body_at(tx, &id, &before, &at).map(|_| ())
            })?;
        }

        // A tier pin goes back to whatever it was, including to no pin at all.
        "work.pinTier" => {
            let id = required(params, "id")?;
            let tier = params
                .get("beforeTier")
                .and_then(Value::as_str)
                .map(str::to_owned);
            let reason = params
                .get("beforeReason")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_owned();
            edit(conn, logged, &at, |tx| match tier {
                Some(tier) => crate::work::pin_tier_at(tx, &id, &tier, &reason, &at).map(|_| ()),
                None => crate::work::unpin_tier_at(tx, &id, &at).map(|_| ()),
            })?;
        }

        // Undoing a creation throws the thing away — into the trash, never
        // outright. Someone can change their mind twice, and a row destroyed by
        // an undo would be gone in a way nothing else in kilna is.
        "work.create" | "work.clone" | "note.create" | "collection.create" | "release.create"
        | "version.create" | "scene.create" | "cut.create" | "comment.create" | "style.create" => {
            let (entity, id) = created(entry)?;
            crate::trash::discard_minted(
                conn,
                entity,
                &id,
                crate::minted::Minted::of(uuid::Uuid::new_v4().to_string(), at.clone()),
                Some(logged.param("at", at.clone())),
            )?;
        }

        // A promotion is taken back whole: the work it made goes to the trash
        // and the note comes back out of it, in one change. Restoring the note
        // alone would leave its text twice — the duplicate promoting exists to
        // prevent.
        "note.promote" => {
            let work_id = required(params, "workId")?;
            let entry_id = required(params, "entryId")?;
            let minted = crate::minted::Minted::of(uuid::Uuid::new_v4().to_string(), at.clone());
            edit(conn, logged, &at, |tx| {
                crate::trash::discard_in_tx(tx, crate::trash::Entity::Work, &work_id, &minted)?;
                crate::trash::restore_in(tx, &entry_id)
            })?;
        }

        // Undoing a deletion is the restore the trash already knows how to do.
        // The entry it names is the one the discard minted.
        "entity.discard" => {
            let entry_id = required(params, "id")?;
            crate::trash::restore(conn, &entry_id, Some(logged.param("at", at.clone())))?;
        }

        // A link is a fact about two works with no body of its own: undoing
        // its making removes it outright, and undoing its removal puts the
        // same row back under the same id and moment, from the copy the
        // deletion recorded.
        "link.create" => {
            let id = crate::minted::Minted::from_params(params)?.id().to_owned();
            edit(conn, logged, &at, |tx| crate::link::delete(tx, &id))?;
        }
        "link.delete" => {
            let id = required(params, "id")?;
            let before = params
                .get("before")
                .and_then(Value::as_object)
                .ok_or_else(|| Error::Other("the deletion recorded no link to put back".into()))?;
            let profile_id = entry
                .profile_id
                .clone()
                .ok_or_else(|| Error::Other("the deletion names no profile".into()))?;
            let new: crate::link::NewLink = serde_json::from_value(Value::Object(before.clone()))?;
            let created_at = before
                .get("created_at")
                .and_then(Value::as_str)
                .unwrap_or(at.as_str())
                .to_owned();
            edit(conn, logged, &at, |tx| {
                crate::link::create_minted(
                    tx,
                    &profile_id,
                    new,
                    crate::minted::Minted::of(id.clone(), created_at.clone()),
                )
                .map(|_| ())
            })?;
        }

        // A release that went out is taken back out of the world. The link it
        // gained is kept, deliberately — see `release::unmark_released`.
        "release.markReleased" => {
            let id = required(params, "id")?;
            edit(conn, logged, &at, |tx| {
                crate::release::unmark_released_at(tx, &id, &at).map(|_| ())
            })?;
        }

        other => {
            return Err(Error::Other(format!(
                "`{other}` says it can be undone but nothing here knows how"
            )));
        }
    }

    Ok(())
}

/// Make a change and record the undo that asked for it, in one transaction.
///
/// The same pairing the commands use: an undo whose log entry survived a failed
/// change would replay into a workspace where the undo happened and the change
/// it takes back did not.
fn edit(
    conn: &mut Connection,
    logged: Intent,
    at: &str,
    change: impl FnOnce(&rusqlite::Transaction<'_>) -> Result<()>,
) -> Result<()> {
    let logged = logged.param("at", at.to_owned());
    let transaction = conn.transaction()?;
    change(&transaction)?;
    operation::record(&transaction, logged)?;
    transaction.commit()?;
    Ok(())
}

/// What a creation created: which table, and under which id.
///
/// The id is the one the operation minted — the whole reason `Minted` exists —
/// so undoing a creation reaches exactly the row it made, even in a workspace
/// rebuilt from the log.
fn created(entry: &Operation) -> Result<(crate::trash::Entity, String)> {
    let entity = match entry.kind.as_str() {
        "work.create" | "work.clone" => crate::trash::Entity::Work,
        "note.create" => crate::trash::Entity::Note,
        "collection.create" => crate::trash::Entity::Collection,
        "release.create" => crate::trash::Entity::Release,
        "version.create" => crate::trash::Entity::Version,
        "scene.create" => crate::trash::Entity::Scene,
        "cut.create" => crate::trash::Entity::Cut,
        "comment.create" => crate::trash::Entity::Comment,
        "style.create" => crate::trash::Entity::Style,
        other => return Err(Error::Other(format!("`{other}` creates nothing"))),
    };
    Ok((entity, required(&entry.params, "id")?))
}

/// A string the operation must carry for its kind to mean anything.
fn required(params: &Map<String, Value>, key: &str) -> Result<String> {
    params
        .get(key)
        .and_then(Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| Error::Other(format!("the operation carries no `{key}`")))
}

/// The operation's payload, read back into the type the domain function takes.
fn from_params<T: serde::de::DeserializeOwned>(
    params: &Map<String, Value>,
    key: &str,
) -> Result<T> {
    let raw = params
        .get(key)
        .ok_or_else(|| Error::Other(format!("the operation carries no `{key}`")))?;
    Ok(serde_json::from_value(raw.clone())?)
}
