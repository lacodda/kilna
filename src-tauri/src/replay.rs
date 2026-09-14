//! Rebuilding a workspace by replaying its operations log.
//!
//! The property this exists to make testable: a database is the sum of the
//! operations that made it, so playing them into an empty workspace produces
//! the same rows. It is the one check that tells a complete log from a
//! plausible one — a missing operation is invisible at the moment it is not
//! written, and shows up only as data that disagrees with itself, much later.
//!
//! Replaying is not an ordinary path through the application. It supplies the
//! ids and timestamps the first run generated (see [`crate::minted`]) instead
//! of minting new ones, and it does not journal, because the journal is a feed
//! a person has already read.
//!
//! Two things it must get right, and both were found by running it rather than
//! by reasoning about it:
//!
//! * **A moment belongs to the gesture, not to the replay.** Every operation
//!   carries the instant its first run used, and the domain functions take it
//!   through the seams. A batch shares one instant across all the rows it
//!   touched, because one gesture happened once — minting a timestamp per row
//!   would put values in the database that the log has no field to hold.
//! * **A derived column is recomputed, not replayed.** `work.status` is a
//!   conclusion drawn from a work's releases and scores, so it is not in the
//!   log and must not be: the same fact stored twice is two facts that can
//!   disagree. [`restate`] recomputes it after each operation, exactly as the
//!   commands do — without it, a rebuild is right in every table and wrong in
//!   the one column a person reads.
//!
//! What a replay refuses to do is guess. A missing value is an error naming the
//! operation, never a fresh one invented in its place: a row rebuilt under an
//! id nothing else names would look original and be wrong, which is the failure
//! this whole scheme exists to prevent. A kind this build does not know is the
//! one soft case — reported as `unknown`, because meeting a newer log is not a
//! fault, and saying so beats a database quietly missing what those operations
//! did.
//!
use rusqlite::Connection;
use serde_json::{Map, Value};

use crate::error::{Error, Result};
use crate::minted::Minted;
use crate::operation::{self, Operation};
use crate::work::version;
use crate::{collection, focus, link, note, profile, release, scene, score, trash, work};

/// What a replay did, and what it could not do.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Report {
    /// Operations played back into the database.
    pub applied: usize,
    /// Operations whose kind this build does not know how to replay, with the
    /// kind named. A log written by a newer build reaches an older one this
    /// way, and saying so is better than silently producing a database that is
    /// missing whatever those operations did.
    pub unknown: Vec<String>,
}

/// Play every operation in `from` into `into`, in the order they happened.
///
/// `into` must be an empty workspace at the current schema version — the point
/// is to rebuild, not to merge, and replaying onto existing rows would collide
/// on ids that are supposed to collide.
pub fn rebuild(from: &Connection, into: &mut Connection) -> Result<Report> {
    let operations = operation::all(from)?;
    let mut report = Report::default();

    for entry in operations {
        match apply(into, &entry) {
            Ok(true) => {
                report.applied += 1;
                restate(into, &entry)?;
            }
            Ok(false) => {
                if !report.unknown.contains(&entry.kind) {
                    report.unknown.push(entry.kind.clone());
                }
            }
            Err(cause) => {
                return Err(Error::Other(format!(
                    "operation `{}` ({}) could not be replayed: {cause}",
                    entry.kind, entry.id
                )));
            }
        }
    }

    Ok(report)
}

/// Bring derived statuses back in line, the way the command that recorded this
/// operation did.
///
/// `work.status` is a conclusion drawn from a work's releases and scores, not a
/// decision anyone made, so the log does not carry it — and must not, or the
/// same fact would be stored in two places that can disagree. Every command that
/// changes one of those facts calls `restate` afterwards; a replay that skipped
/// it rebuilds every table correctly and leaves the one column a person actually
/// reads showing what the work was before its release went out.
///
/// The moment comes from the operation, never from the clock — both a stamping
/// operation and a minting one carry it under `at`. A status recomputed at the
/// hour of the rebuild would stamp `updated_at` with a time that never happened.
///
/// Recomputing the whole profile rather than the works this operation touched is
/// deliberate. `resync_at` writes only where the derivation disagrees with the
/// stored word, so the result matches calling `refresh` on each affected work —
/// without this module keeping its own copy of which command touches which work,
/// a copy that would go quietly stale.
fn restate(conn: &Connection, entry: &Operation) -> Result<()> {
    let Some(at) = entry.params.get("at").and_then(Value::as_str) else {
        // No moment: an operation that changed no fact a status is derived from.
        return Ok(());
    };
    // Resolved through the key, not through `entry.profile_id`: that id belongs
    // to the workspace the log came from, and this one mints its own.
    let profile_id = workspace_profile(conn, &entry.params)?;

    let config = crate::profile::config_for(conn, &profile_id)?;
    crate::work::status::resync_at(conn, &config, &profile_id, at)?;
    Ok(())
}

/// Apply one operation. `false` means this build does not know the kind.
///
/// Two kinds are deliberately absent from the match below rather than made to
/// look replayable: see the module-level list of what a replay cannot yet
/// rebuild byte-for-byte, in the doc comment on [`rebuild`].
fn apply(conn: &mut Connection, entry: &Operation) -> Result<bool> {
    let params = &entry.params;

    match entry.kind.as_str() {
        "work.create" => {
            let profile_id = workspace_profile(conn, params)?;
            let new = from_params(params, "work")?;
            work::create_minted(conn, &profile_id, new, minted(params)?)?;
        }

        "work.update" => {
            let id = required(params, "id")?;
            let patch = from_params(params, "patch")?;
            let at = required(params, "at")?;
            work::update_at(conn, &id, patch, &at)?;
        }

        "work.pinTier" => {
            let id = required(params, "id")?;
            let tier = required(params, "tier")?;
            let reason = required(params, "reason")?;
            let at = required(params, "at")?;
            work::pin_tier_at(conn, &id, &tier, &reason, &at)?;
        }

        "work.unpinTier" => {
            let id = required(params, "id")?;
            let at = required(params, "at")?;
            work::unpin_tier_at(conn, &id, &at)?;
        }

        "work.unpinStatus" => {
            let profile_id = workspace_profile(conn, params)?;
            let config = profile::config_for(conn, &profile_id)?;
            let id = required(params, "id")?;
            let at = required(params, "at")?;
            work::status::unpin_at(conn, &config, &id, &at)?;
        }

        "work.setStatusBatch" => {
            let work_ids: Vec<String> = from_params(params, "workIds")?;
            let status = required(params, "status")?;
            let at = required(params, "at")?;
            // One gesture moved every listed work at once — replayed the same
            // way, in a loop, so the batch lands on the same rows it did live.
            for work_id in &work_ids {
                let patch = work::WorkPatch {
                    status: Some(status.clone()),
                    ..work::WorkPatch::default()
                };
                work::update_at(conn, work_id, patch, &at)?;
            }
        }

        "status.resync" => {
            let profile_id = workspace_profile(conn, params)?;
            let config = profile::config_for(conn, &profile_id)?;
            let at = required(params, "at")?;
            work::status::resync_at(conn, &config, &profile_id, &at)?;
        }

        "version.create" => {
            let work_id = required(params, "workId")?;
            let new = from_params(params, "version")?;
            version::create_minted(conn, &work_id, new, minted(params)?, None)?;
        }

        "note.create" => {
            let profile_id = workspace_profile(conn, params)?;
            let new = from_params(params, "note")?;
            note::create_minted(conn, &profile_id, new, minted(params)?)?;
        }

        "note.update" => {
            let id = required(params, "id")?;
            let patch = from_params(params, "patch")?;
            let at = required(params, "at")?;
            note::update_at(conn, &id, patch, &at)?;
        }

        "version.edit" => {
            let id = required(params, "id")?;
            let body = required(params, "body")?;
            let at = required(params, "at")?;
            version::update_body_at(conn, &id, &body, &at)?;
        }

        "finding.dismiss" => {
            let profile_id = workspace_profile(conn, params)?;
            let key = from_params(params, "key")?;
            let at = required(params, "at")?;
            focus::dismiss_at(conn, &profile_id, &key, &at)?;
        }

        "finding.restore" => {
            let profile_id = workspace_profile(conn, params)?;
            let key = from_params(params, "key")?;
            focus::restore(conn, &profile_id, &key)?;
        }

        "focusNote.create" => {
            let profile_id = workspace_profile(conn, params)?;
            let new = from_params(params, "note")?;
            focus::add_note_minted(conn, &profile_id, new, minted(params)?)?;
        }

        "focusNote.update" => {
            let id = required(params, "id")?;
            let patch = from_params(params, "patch")?;
            let at = required(params, "at")?;
            focus::update_note_at(conn, &id, patch, &at)?;
        }

        "focusNote.reorder" => {
            let profile_id = workspace_profile(conn, params)?;
            let order: Vec<String> = from_params(params, "order")?;
            focus::reorder_notes(conn, &profile_id, &order, None)?;
        }

        "focusNote.delete" => {
            let id = required(params, "id")?;
            focus::delete_note(conn, &id)?;
        }

        "score.create" => {
            let work_id = required(params, "workId")?;
            let new = from_params(params, "score")?;
            score::create_minted(conn, &work_id, new, minted(params)?)?;
        }

        "release.create" => {
            let new = from_params(params, "release")?;
            release::create_minted(conn, new, minted(params)?)?;
        }

        "release.update" => {
            let id = required(params, "id")?;
            let patch = from_params(params, "patch")?;
            let at = required(params, "at")?;
            release::update_at(conn, &id, patch, &at)?;
        }

        "release.schedule" => {
            let id = required(params, "id")?;
            let slot = required(params, "slot")?;
            let at = required(params, "at")?;
            release::schedule_at(conn, &id, &slot, &at)?;
        }

        "release.setSlotPin" => {
            let id = required(params, "id")?;
            let pinned = params
                .get("pinned")
                .and_then(Value::as_bool)
                .ok_or_else(|| Error::Other("the operation carries no `pinned`".into()))?;
            let at = required(params, "at")?;
            release::set_slot_pin_at(conn, &id, pinned, &at)?;
        }

        "release.unschedule" => {
            let id = required(params, "id")?;
            let at = required(params, "at")?;
            release::unschedule_at(conn, &id, &at)?;
        }

        "release.unscheduleBatch" => {
            let release_ids: Vec<String> = from_params(params, "releaseIds")?;
            let at = required(params, "at")?;
            // One gesture took every listed release off the calendar at once —
            // replayed the same way, in a loop.
            for release_id in &release_ids {
                release::unschedule_at(conn, release_id, &at)?;
            }
        }

        "release.markReleased" => {
            let id = required(params, "id")?;
            let url = params.get("url").and_then(Value::as_str).map(str::to_owned);
            let on_day = params
                .get("on_day")
                .and_then(Value::as_str)
                .map(str::to_owned);
            let at = required(params, "at")?;
            release::mark_released_at(conn, &id, url, on_day, &at)?;
        }

        "release.unmarkReleased" => {
            let id = required(params, "id")?;
            let at = required(params, "at")?;
            release::unmark_released_at(conn, &id, &at)?;
        }

        "collection.create" => {
            let profile_id = workspace_profile(conn, params)?;
            let new = from_params(params, "collection")?;
            collection::create_minted(conn, &profile_id, new, minted(params)?)?;
        }

        "collection.update" => {
            let id = required(params, "id")?;
            let patch = from_params(params, "patch")?;
            let at = required(params, "at")?;
            collection::update_at(conn, &id, patch, &at)?;
        }

        "collection.setContents" => {
            let id = required(params, "id")?;
            let work_ids: Vec<String> = from_params(params, "workIds")?;
            let at = required(params, "at")?;
            collection::set_contents_at(conn, &id, &work_ids, &at, None)?;
        }

        "link.create" => {
            let profile_id = workspace_profile(conn, params)?;
            let new = from_params(params, "link")?;
            link::create_minted(conn, &profile_id, new, minted(params)?)?;
        }

        "link.delete" => {
            let id = required(params, "id")?;
            link::delete(conn, &id)?;
        }

        "scene.create" => {
            let profile_id = workspace_profile(conn, params)?;
            let new = from_params(params, "scene")?;
            scene::create_minted(conn, &profile_id, new, minted(params)?)?;
        }

        "scene.update" => {
            let id = required(params, "id")?;
            let patch = from_params(params, "patch")?;
            let at = required(params, "at")?;
            scene::update_at(conn, &id, patch, &at)?;
        }

        "scene.time" => {
            let work_id = required(params, "workId")?;
            let at = required(params, "at")?;
            scene::time_board_at(conn, &work_id, &at, None)?;
        }

        "scene.attachNote" => {
            let scene_id = required(params, "sceneId")?;
            let note_id = required(params, "noteId")?;
            crate::scene_note::attach_minted(conn, &scene_id, &note_id, minted(params)?)?;
        }

        "scene.detachNote" => {
            let scene_id = required(params, "sceneId")?;
            let note_id = required(params, "noteId")?;
            crate::scene_note::detach(conn, &scene_id, &note_id)?;
        }

        "scene.frame" => {
            let work_id = required(params, "workId")?;
            let role = required(params, "role")?;
            let at = required(params, "at")?;
            let ids: Vec<String> = from_params(params, "ids")?;
            // The ids the first run minted, so the rebuilt board is the same
            // board — see `Minted` on why a replacement is refused instead.
            let minted: Vec<crate::minted::Minted> = ids
                .into_iter()
                .map(|id| crate::minted::Minted::of(id, at.clone()))
                .collect();
            scene::frame_from_text(conn, &work_id, &role, &minted, None)?;
        }

        "entity.discard" => {
            let entity = trash::Entity::parse(&required(params, "entity")?)?;
            let id = required(params, "entityId")?;
            trash::discard_minted(conn, entity, &id, minted(params)?, None)?;
        }

        "trash.restore" => {
            let id = required(params, "id")?;
            trash::restore(conn, &id, None)?;
        }

        "trash.purge" => {
            let id = required(params, "id")?;
            trash::purge(conn, &id, None)?;
        }

        "trash.empty" => {
            let profile_id = workspace_profile(conn, params)?;
            trash::empty(conn, &profile_id)?;
        }

        // "work.discardBatch" and "layout.apply" are not replayed: see the
        // module doc comment above for what their params are missing. Both
        // fall through to the `unknown` arm below rather than being played
        // with a guessed value.
        "work.discardBatch" => {
            // The entry ids are paired with the work ids by position, and every
            // entry of one gesture shares its moment — see the command.
            let work_ids: Vec<String> = from_params(params, "workIds")?;
            let entry_ids: Vec<String> = from_params(params, "entryIds")?;
            let at = required(params, "at")?;
            if work_ids.len() != entry_ids.len() {
                return Err(Error::Other(
                    "the batch names a different number of works and trash entries".into(),
                ));
            }
            let minted: Vec<Minted> = entry_ids
                .iter()
                .map(|id| Minted::of(id.clone(), at.clone()))
                .collect();
            crate::trash::discard_works_batch(conn, &work_ids, &minted, None)?;
        }

        "layout.apply" => {
            let placements: Vec<crate::layout::Placement> = from_params(params, "placements")?;
            let at = required(params, "at")?;
            crate::layout::apply_at(conn, &placements, None, &at)?;
        }

        _ => return Ok(false),
    }

    Ok(true)
}

/// The id this workspace gives the profile the operation names.
///
/// The log carries the key, because ids are minted per workspace — see ADR
/// 0014. A key this workspace does not have is an error rather than a skip: the
/// operations under it would all land in the wrong place or nowhere.
fn workspace_profile(conn: &Connection, params: &Map<String, Value>) -> Result<String> {
    let key = required(params, "profile")?;
    profile::id_for_key(conn, &key)?
        .ok_or_else(|| Error::Other(format!("this workspace has no profile `{key}`")))
}

/// The generated values this operation recorded.
fn minted(params: &Map<String, Value>) -> Result<Minted> {
    Minted::from_params(params)
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
