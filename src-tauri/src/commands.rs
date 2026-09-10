use std::sync::Arc;

use tauri::{AppHandle, Emitter, Manager, State};

use crate::assistant::run::{self as assistant_run, Emission, Run, Sink};
use crate::assistant::{self, Chat, Message, NewChat, Transcript, cli, prompt};
use crate::collection::{self, Collection, CollectionPatch, NewCollection};
use crate::error::{Error, Result};
use crate::exchange::backup;
use crate::exchange::export::{self, ExportReport};
use crate::exchange::import::{self, ImportReport};
use crate::focus::{self, Dismissal, DismissalKey, FocusNote, FocusNotePatch, NewFocusNote};
use crate::journal::{self, Entry, Record};
use crate::layout;
use crate::minted::Minted;
use crate::note::{self, NewNote, Note, NoteFilter, NotePatch};
use crate::operation;
use crate::plugin::{self, manifest::Plugin, manifest::Target};
use crate::profile::{self, Profile, Workspace};
use crate::release::{self, NewRelease, Release, ReleasePatch, ScheduledRelease, Scheduling};
use crate::reversal;
use crate::score::{self, NewScore, Score, ScoredWork};
use crate::search::{self, Hit};
use crate::state::AppState;
use crate::time;
use crate::trash::{self, Deletion};
use crate::undo;
use crate::work::version::{self, NewVersion, Version, VersionSummary};
use crate::work::{self, NewWork, Work, WorkFilter, WorkPatch};

/// Everything the status screen needs in one round trip.
#[tauri::command]
pub fn get_workspace(state: State<'_, AppState>) -> Result<Workspace> {
    let conn = state.conn();
    profile::workspace(&conn)
}

#[tauri::command]
pub fn list_profiles(state: State<'_, AppState>) -> Result<Vec<Profile>> {
    let conn = state.conn();
    profile::list(&conn)
}

#[tauri::command]
pub fn activate_profile(state: State<'_, AppState>, id: String) -> Result<()> {
    let mut conn = state.conn();
    profile::activate(&mut conn, &id)
}

/// Replace a profile's configuration.
///
/// Existing works keep their status and kind even when the vocabulary that
/// named them is edited away — an old value stays visible rather than being
/// rewritten, because the alternative is silently changing what a work is.
#[tauri::command]
pub fn update_profile_config(
    state: State<'_, AppState>,
    id: String,
    config: profile::config::ProfileConfig,
) -> Result<Profile> {
    let conn = state.conn();
    profile::update_config(&conn, &id, &config)
}

/// Do something to the workspace and record the operation that asked for it, in
/// one transaction.
///
/// Every mutating command goes through here, so that the log entry and the rows
/// it describes arrive together or not at all. Writing the operation beside the
/// change instead would leave, on the one failure in a thousand, a log that
/// replays into a database that never existed — the failure this whole stage is
/// against. See ADR 0014.
///
/// The closure gets the transaction, not the connection: a domain function that
/// opens its own cannot be used here, and takes the operation as an argument
/// instead — see `trash::discard_minted`.
fn recording<T>(
    conn: &mut rusqlite::Connection,
    logged: operation::Intent,
    change: impl FnOnce(&rusqlite::Transaction<'_>) -> Result<T>,
) -> Result<T> {
    let transaction = conn.transaction()?;
    let done = change(&transaction)?;
    operation::record(&transaction, logged)?;
    transaction.commit()?;
    Ok(done)
}

/// What the fields a patch names held before it was applied.
///
/// Recorded beside the patch so that an undo has something to put back. Only
/// the patched fields: reverting a rename must not also revert a status
/// somebody set in between — see [`crate::reversal`], which holds the rule and
/// the tests for it.
///
/// A row that is not there yields an empty object rather than an error. The
/// change about to be attempted will fail on its own and say so properly; a log
/// helper is not the place to decide that.
fn was<T: serde::Serialize, P: serde::Serialize>(
    before: Option<&T>,
    patch: &P,
) -> Result<serde_json::Value> {
    let (Some(before), Ok(serde_json::Value::Object(patch))) =
        (before, serde_json::to_value(patch))
    else {
        return Ok(serde_json::Value::Object(serde_json::Map::new()));
    };
    let serde_json::Value::Object(before) = serde_json::to_value(before)? else {
        return Ok(serde_json::Value::Object(serde_json::Map::new()));
    };
    Ok(serde_json::Value::Object(reversal::invert(&before, &patch)))
}

/// The stable key of a profile, for the operations log.
///
/// Operations name profiles by key rather than by id: an id is minted per
/// workspace, so a log replayed into another copy would look for a profile that
/// is not there under that name. See ADR 0014.
fn profile_key(conn: &rusqlite::Connection, profile_id: &str) -> Result<String> {
    profile::key_for_id(conn, profile_id)?.ok_or_else(|| Error::not_found("profile", profile_id))
}

/// The id of the active profile, or an error the frontend can show.
///
/// Every work and note command is scoped to it: the frontend never has to carry
/// the profile id around, and a request cannot land in the wrong profile.
fn active_profile_id(conn: &rusqlite::Connection) -> Result<String> {
    profile::active(conn)?
        .map(|profile| profile.id)
        .ok_or_else(|| Error::Other("no active profile".into()))
}

/// Bring a work's status back in line with the facts, and say so.
///
/// Called from every command that changes one of the facts a status is derived
/// from. Deriving here rather than inside each of those is the whole point: the
/// predecessor wrote the field from four places and it drifted apart from what
/// was true. A pinned work is left alone by [`work::status::refresh`] itself,
/// so a call site never has to remember to check.
fn restate(conn: &rusqlite::Connection, profile_id: &str, work_id: &str) {
    let config = match profile::config_for(conn, profile_id) {
        Ok(config) => config,
        Err(cause) => {
            eprintln!("status: could not read the profile: {cause}");
            return;
        }
    };

    match work::status::refresh(conn, &config, work_id) {
        Ok(Some(change)) => journal::record(
            conn,
            profile_id,
            Record::new("work.restated")
                .param("title", change.title)
                .param("from", change.from)
                .param("to", change.to)
                .about("work", work_id.to_owned()),
        ),
        Ok(None) => {}
        Err(cause) => eprintln!("status: could not restate the work: {cause}"),
    }
}

#[tauri::command]
pub fn list_works(state: State<'_, AppState>, filter: Option<WorkFilter>) -> Result<Vec<Work>> {
    let conn = state.conn();
    let profile_id = active_profile_id(&conn)?;
    work::list(&conn, &profile_id, &filter.unwrap_or_default())
}

#[tauri::command]
pub fn get_work(state: State<'_, AppState>, id: String) -> Result<Option<Work>> {
    let conn = state.conn();
    work::get(&conn, &id)
}

#[tauri::command]
pub fn create_work(state: State<'_, AppState>, work: NewWork) -> Result<Work> {
    let mut conn = state.conn();
    let profile_id = active_profile_id(&conn)?;

    // Minted out here, not inside `work::create`, so the same id and timestamp
    // reach both the row and the log: a replay rebuilds the work under the id
    // that every version, score and release already names. The two writes share
    // a transaction — a log entry for a work that failed to insert would replay
    // into a row that never existed. See ADR 0014.
    let minted = Minted::fresh();
    let logged = operation::Intent::new("work.create")
        .in_profile(&profile_id)
        .param("profile", profile_key(&conn, &profile_id)?)
        .param("work", serde_json::to_value(&work)?)
        .minted(&minted);

    let created = recording(&mut conn, logged, |tx| {
        work::create_minted(tx, &profile_id, work, minted)
    })?;

    journal::record(
        &conn,
        &profile_id,
        Record::new("work.created")
            .param("title", created.title.clone())
            .about("work", created.id.clone()),
    );

    Ok(created)
}

/// Edit a work.
///
/// Renaming and moving to another status are recorded separately, and with both
/// values: "renamed to X" without the old name is a line nobody can act on, and
/// a status change is the one edit the calendar and the catalogue both react to.
#[tauri::command]
pub fn update_work(state: State<'_, AppState>, id: String, patch: WorkPatch) -> Result<Work> {
    let mut conn = state.conn();
    let profile_id = active_profile_id(&conn)?;
    let before = work::get(&conn, &id)?;

    // The moment is decided here so the row and the log agree on it: an edit
    // replayed at a different instant leaves a different `updated_at`, and the
    // rebuilt database stops matching. See ADR 0014.
    let at = time::now();
    let logged = operation::Intent::new("work.update")
        .in_profile(&profile_id)
        .param("profile", profile_key(&conn, &profile_id)?)
        .param("id", id.clone())
        .param("patch", serde_json::to_value(&patch)?)
        .param("before", was(before.as_ref(), &patch)?)
        .param("at", at.clone());

    let updated = recording(&mut conn, logged, |tx| work::update_at(tx, &id, patch, &at))?;

    if let Some(before) = before {
        if before.title != updated.title {
            journal::record(
                &conn,
                &profile_id,
                Record::new("work.renamed")
                    .param("from", before.title)
                    .param("to", updated.title.clone())
                    .about("work", updated.id.clone()),
            );
        }
        if before.status != updated.status {
            journal::record(
                &conn,
                &profile_id,
                Record::new("work.status")
                    .param("title", updated.title.clone())
                    .param("from", before.status)
                    .param("to", updated.status.clone())
                    .about("work", updated.id.clone()),
            );
        }
    }

    Ok(updated)
}

/// Move a work to the trash. Returns the entry id, which is what an undo needs.
///
/// Nothing in the app deletes outright: every `delete_*` command below goes the
/// same way, so an undo is always available and a confirmation never is.
/// What a full recompute would change, changing nothing.
///
/// The dry run is the whole reason a mass restate is safe to offer: a profile
/// whose `derive` roles are wrong would otherwise silently rewrite the status
/// of every work in the workspace, and there is no undo for that.
#[tauri::command]
pub fn status_drift(state: State<'_, AppState>) -> Result<Vec<work::status::Change>> {
    let conn = state.conn();
    let profile_id = active_profile_id(&conn)?;
    let config = profile::config_for(&conn, &profile_id)?;
    work::status::drift(&conn, &config, &profile_id)
}

/// Apply what [`status_drift`] reported.
#[tauri::command]
pub fn resync_statuses(state: State<'_, AppState>) -> Result<Vec<work::status::Change>> {
    let mut conn = state.conn();
    let profile_id = active_profile_id(&conn)?;
    let config = profile::config_for(&conn, &profile_id)?;

    let at = time::now();
    let logged = operation::Intent::new("status.resync")
        .in_profile(&profile_id)
        .param("profile", profile_key(&conn, &profile_id)?)
        .param("at", at.clone());

    let changes = recording(&mut conn, logged, |tx| {
        work::status::resync_at(tx, &config, &profile_id, &at)
    })?;

    if !changes.is_empty() {
        journal::record(
            &conn,
            &profile_id,
            Record::new("status.resynced").param("count", changes.len() as i64),
        );
    }

    Ok(changes)
}

/// Hand one work's status back to the automation.
#[tauri::command]
pub fn unpin_status(state: State<'_, AppState>, id: String) -> Result<Work> {
    let mut conn = state.conn();
    let profile_id = active_profile_id(&conn)?;
    let config = profile::config_for(&conn, &profile_id)?;

    let at = time::now();
    let logged = operation::Intent::new("work.unpinStatus")
        .in_profile(&profile_id)
        .param("profile", profile_key(&conn, &profile_id)?)
        .param("id", id.clone())
        .param("at", at.clone());

    let change = recording(&mut conn, logged, |tx| {
        work::status::unpin_at(tx, &config, &id, &at)
    })?;

    if let Some(change) = change {
        journal::record(
            &conn,
            &profile_id,
            Record::new("work.restated")
                .param("title", change.title)
                .param("from", change.from)
                .param("to", change.to)
                .about("work", id.clone()),
        );
    }

    work::get(&conn, &id)?.ok_or_else(|| Error::not_found("work", &id))
}

/// Hold a work at a tier by hand, with the reason on record.
#[tauri::command]
pub fn pin_tier(
    state: State<'_, AppState>,
    id: String,
    tier: String,
    reason: String,
) -> Result<Work> {
    let mut conn = state.conn();
    let profile_id = active_profile_id(&conn)?;
    let before = work::get(&conn, &id)?;

    let at = time::now();
    // The prior pin travels with the intent so an undo can put it back exactly
    // as it stood — `null` means there was no pin to restore. See ADR 0014.
    let (before_tier, before_reason) =
        before
            .as_ref()
            .map_or((serde_json::Value::Null, serde_json::Value::Null), |work| {
                (
                    serde_json::to_value(&work.tier_pinned).unwrap_or(serde_json::Value::Null),
                    serde_json::to_value(&work.tier_pin_reason).unwrap_or(serde_json::Value::Null),
                )
            });
    let logged = operation::Intent::new("work.pinTier")
        .in_profile(&profile_id)
        .param("profile", profile_key(&conn, &profile_id)?)
        .param("id", id.clone())
        .param("tier", tier.clone())
        .param("reason", reason.clone())
        .param("beforeTier", before_tier)
        .param("beforeReason", before_reason)
        .param("at", at.clone());

    let pinned = recording(&mut conn, logged, |tx| {
        work::pin_tier_at(tx, &id, &tier, &reason, &at)
    })?;

    journal::record(
        &conn,
        &profile_id,
        Record::new("tier.pinned")
            .param("title", pinned.title.clone())
            .param("tier", tier)
            .param("reason", reason.trim().to_owned())
            .about("work", id),
    );
    Ok(pinned)
}

/// Let the score speak for the work's tier again.
#[tauri::command]
pub fn unpin_tier(state: State<'_, AppState>, id: String) -> Result<Work> {
    let mut conn = state.conn();
    let profile_id = active_profile_id(&conn)?;

    let at = time::now();
    let logged = operation::Intent::new("work.unpinTier")
        .in_profile(&profile_id)
        .param("profile", profile_key(&conn, &profile_id)?)
        .param("id", id.clone())
        .param("at", at.clone());

    let freed = recording(&mut conn, logged, |tx| work::unpin_tier_at(tx, &id, &at))?;

    journal::record(
        &conn,
        &profile_id,
        Record::new("tier.unpinned")
            .param("title", freed.title.clone())
            .about("work", id),
    );
    Ok(freed)
}

#[tauri::command]
pub fn delete_work(state: State<'_, AppState>, id: String) -> Result<String> {
    discard_and_record(&state, trash::Entity::Work, &id)
}

/// What a bulk edit did. Counted rather than returned row by row: the catalogue
/// reloads afterwards anyway, and what a person wants to be told is how many.
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BulkOutcome {
    /// Works the change actually landed on.
    pub changed: usize,
    /// Works that were already that way, or had nothing to unschedule. Not an
    /// error: in a batch this is the reason the number is smaller than the
    /// selection, and saying so is kinder than silence.
    pub skipped: usize,
}

/// Move several works to another status at once.
///
/// Chosen by hand, so each one pins exactly as the single-work path does —
/// otherwise the automation would derive the old status straight back and the
/// batch would appear to do nothing.
///
/// One journal line for the batch, not one per work: a person who moved twenty
/// drafts did one thing, and twenty lines would bury the day's real events.
#[tauri::command]
pub fn set_works_status(
    state: State<'_, AppState>,
    work_ids: Vec<String>,
    status: String,
) -> Result<BulkOutcome> {
    let mut conn = state.conn();
    let profile_id = active_profile_id(&conn)?;

    // Refused before anything changes rather than once per work: a status the
    // profile does not have is a mistake about the whole batch.
    let config = profile::config_for(&conn, &profile_id)?;
    if !config.statuses.iter().any(|known| known.key == status) {
        return Err(Error::not_found("status", &status));
    }

    // The moment is decided here so every work in the batch, and the log
    // entry describing it, agree on it. See ADR 0014.
    let at = time::now();
    let mut moved: Vec<String> = Vec::new();
    let mut skipped = 0usize;

    {
        let tx = conn.transaction()?;

        for work_id in &work_ids {
            let before = work::get(&tx, work_id)?;
            if before.as_ref().is_some_and(|work| work.status == status) {
                skipped += 1;
                continue;
            }

            let patch = WorkPatch {
                status: Some(status.clone()),
                ..WorkPatch::default()
            };

            // One work failing must not take the batch with it — the others
            // are unrelated, and a half-applied batch is more useful than
            // none.
            match work::update_at(&tx, work_id, patch, &at) {
                Ok(_) => moved.push(work_id.clone()),
                Err(cause) => {
                    eprintln!("status: {work_id} could not be moved: {cause}");
                    skipped += 1;
                }
            }
        }

        // Only the works actually moved go in the log: a work already at this
        // status was skipped, and recording it would replay `updated_at` onto
        // a row a repeat run never touched.
        if !moved.is_empty() {
            let logged = operation::Intent::new("work.setStatusBatch")
                .in_profile(&profile_id)
                .param("profile", profile_key(&tx, &profile_id)?)
                .param("workIds", serde_json::to_value(&moved)?)
                .param("status", status.clone())
                .param("at", at.clone());
            operation::record(&tx, logged)?;
        }

        tx.commit()?;
    }

    let changed = moved.len();
    if changed > 0 {
        journal::record(
            &conn,
            &profile_id,
            Record::new("work.statusBatch")
                .param("to", status)
                .param("count", i64::try_from(changed).unwrap_or(i64::MAX)),
        );
    }

    Ok(BulkOutcome { changed, skipped })
}

/// Take several works off the calendar at once.
///
/// Only planned releases holding a slot are touched; anything already out stays
/// where it is. Each work is restated afterwards, so a work that has nothing
/// booked any more goes back to saying so on its own.
#[tauri::command]
pub fn unschedule_works(state: State<'_, AppState>, work_ids: Vec<String>) -> Result<BulkOutcome> {
    let mut conn = state.conn();
    let profile_id = active_profile_id(&conn)?;

    // The moment is decided here so every release taken off the calendar, and
    // the log entry describing it, agree on it. See ADR 0014.
    let at = time::now();
    let mut released_ids: Vec<String> = Vec::new();
    let mut skipped = 0usize;

    {
        let tx = conn.transaction()?;

        for work_id in &work_ids {
            let scheduled = release::scheduled_for(&tx, work_id)?;
            if scheduled.is_empty() {
                skipped += 1;
                continue;
            }

            for release_id in scheduled {
                match release::unschedule_at(&tx, &release_id, &at) {
                    Ok(_) => released_ids.push(release_id),
                    Err(cause) => {
                        eprintln!("calendar: {release_id} could not be unscheduled: {cause}");
                        skipped += 1;
                    }
                }
            }
        }

        // Only the releases actually taken off the calendar go in the log: a
        // work with nothing booked was skipped, and recording it would replay
        // an unschedule onto a release a repeat run never touched.
        if !released_ids.is_empty() {
            let logged = operation::Intent::new("release.unscheduleBatch")
                .in_profile(&profile_id)
                .param("profile", profile_key(&tx, &profile_id)?)
                .param("releaseIds", serde_json::to_value(&released_ids)?)
                .param("at", at.clone());
            operation::record(&tx, logged)?;
        }

        tx.commit()?;
    }

    for work_id in &work_ids {
        restate(&conn, &profile_id, work_id);
    }

    let changed = released_ids.len();
    if changed > 0 {
        journal::record(
            &conn,
            &profile_id,
            Record::new("release.unscheduledBatch")
                .param("count", i64::try_from(changed).unwrap_or(i64::MAX)),
        );
    }

    Ok(BulkOutcome { changed, skipped })
}

/// Move several works to the trash at once.
///
/// Each still gets its own trash entry, because an entry snapshots one entity —
/// so each is separately restorable, and the frontend offers a single undo
/// across the batch. What this replaces is the loop of separate calls the
/// catalogue used to make, which left the journal with one line per work and
/// could stop halfway with no sign of where.
#[tauri::command]
pub fn delete_works(state: State<'_, AppState>, ids: Vec<String>) -> Result<Vec<String>> {
    let mut conn = state.conn();
    let profile_id = active_profile_id(&conn)?;

    // Titles are read before anything is discarded — afterwards there is
    // nothing left to ask.
    let titles: Vec<Option<String>> = ids
        .iter()
        .map(|id| journal::work_title(&conn, id))
        .collect();

    // One minted trash entry per work, one operation for the whole batch: an
    // undo has one gesture to reverse, not `ids.len()` of them. See ADR 0014.
    //
    // All of them share one moment. Minting each entry with its own `now()`
    // would put `ids.len()` different timestamps into a single operation, and
    // the log has one field to keep them in — so a rebuild would have to invent
    // the rest. One gesture happened at one time; the entries say so.
    let at = time::now();
    let minted_ids: Vec<Minted> = ids
        .iter()
        .map(|_| Minted::of(uuid::Uuid::new_v4().to_string(), at.clone()))
        .collect();
    let entry_ids: Vec<String> = minted_ids.iter().map(|m| m.id().to_owned()).collect();
    let logged = operation::Intent::new("work.discardBatch")
        .in_profile(&profile_id)
        .param("profile", profile_key(&conn, &profile_id)?)
        .param("workIds", serde_json::to_value(&ids)?)
        .param("entryIds", serde_json::to_value(&entry_ids)?)
        .param("at", at.clone());

    let discarded = trash::discard_works_batch(&mut conn, &ids, &minted_ids, Some(logged))?;
    let entries: Vec<String> = discarded.iter().map(|(_, entry)| entry.clone()).collect();

    if discarded.len() == 1 {
        let (id, _) = &discarded[0];
        let title = ids
            .iter()
            .position(|candidate| candidate == id)
            .and_then(|index| titles[index].clone());
        journal::record(
            &conn,
            &profile_id,
            Record::new("work.deleted")
                .param("title", title.unwrap_or_else(|| id.clone()))
                .about("work", id.clone()),
        );
    } else if discarded.len() > 1 {
        journal::record(
            &conn,
            &profile_id,
            Record::new("work.deletedBatch")
                .param("count", i64::try_from(discarded.len()).unwrap_or(i64::MAX)),
        );
    }

    Ok(entries)
}

#[tauri::command]
pub fn list_versions(state: State<'_, AppState>, work_id: String) -> Result<Vec<VersionSummary>> {
    let conn = state.conn();
    version::list(&conn, &work_id)
}

#[tauri::command]
pub fn get_version(state: State<'_, AppState>, id: String) -> Result<Option<Version>> {
    let conn = state.conn();
    version::get(&conn, &id)
}

#[tauri::command]
pub fn create_version(
    state: State<'_, AppState>,
    work_id: String,
    version: NewVersion,
) -> Result<Version> {
    let mut conn = state.conn();
    let profile_id = active_profile_id(&conn)?;

    // Minted here, and carried into `create_minted`, so a replay lands the
    // version under the id everything else already names. The operation is
    // written inside that function's own transaction — see its doc comment
    // and ADR 0014.
    let minted = Minted::fresh();
    let logged = operation::Intent::new("version.create")
        .in_profile(&profile_id)
        .param("profile", profile_key(&conn, &profile_id)?)
        .param("workId", work_id.clone())
        .param("version", serde_json::to_value(&version)?)
        .minted(&minted);

    let created = version::create_minted(&mut conn, &work_id, version, minted, Some(logged))?;

    journal::record(
        &conn,
        &profile_id,
        Record::new("version.created")
            .param("role", created.role.clone())
            .param("revision", created.revision)
            .param(
                "title",
                journal::work_title(&conn, &work_id).unwrap_or_default(),
            )
            // Filed under the work rather than the version: the History tab
            // people want is the work's, and a version's own tab would hold one
            // line saying it was made.
            .about("work", work_id.clone()),
    );

    Ok(created)
}

#[tauri::command]
pub fn set_current_version(
    state: State<'_, AppState>,
    work_id: String,
    version_id: String,
) -> Result<()> {
    let conn = state.conn();
    version::set_current(&conn, &work_id, &version_id)
}

/// Move something to the trash and write one line about it.
///
/// Every `delete_*` below is this call with a different entity, for the same
/// reason `announceDeleted` is one function on the other side: six near-copies
/// are six chances for one of them to stop recording and nobody to notice.
///
/// The label is read before the deletion, while there is still something to
/// read. Which work it belonged to comes from the trash entry, which has just
/// snapshotted it.
fn discard_and_record(
    state: &State<'_, AppState>,
    entity: trash::Entity,
    id: &str,
) -> Result<String> {
    let mut conn = state.conn();
    let profile_id = active_profile_id(&conn)?;

    // One operation for all six entities, minted here so that the trash entry a
    // restore names is the same one after a rebuild. It travels into `discard`
    // rather than being written around it, so the log and the deletion share a
    // transaction. See ADR 0014.
    let minted = Minted::fresh();
    let logged = operation::Intent::new("entity.discard")
        .in_profile(&profile_id)
        .param("profile", profile_key(&conn, &profile_id)?)
        .param("entity", entity.as_str())
        .param("entityId", id)
        .minted(&minted);

    // `trash::discard_minted` opens its own transaction, so `logged` travels in
    // as an argument instead of through `recording`: the call below reaches
    // `operation::record` one level down, inside that same transaction.
    let entry_id = trash::discard_minted(&mut conn, entity, id, minted, Some(logged))?;

    let described = trash::list(&conn, &profile_id)?
        .into_iter()
        .find(|entry| entry.id == entry_id);

    // The key follows the entity — `score` deleted is `score.deleted` — so the
    // six deletions cannot each invent their own wording, and the test that
    // checks every recorded key has a sentence has one place to look.
    let mut record = Record::new(format!("{}.deleted", entity.as_str())).param(
        "label",
        described
            .as_ref()
            .map_or_else(|| id.to_owned(), |entry| entry.label.clone()),
    );
    // Filed under the work it came from, so the card's History tab shows what
    // was taken out of it — a deleted score is part of that work's story.
    if let Some(origin) = described.as_ref().and_then(|entry| entry.origin.clone()) {
        record = record.param("origin", origin);
    }
    let behind = work_behind(&conn, entity, id);
    if let Some(work_id) = behind.clone() {
        record = record.about("work", work_id);
    }

    journal::record(&conn, &profile_id, record);

    // Deleting the last score, or the release that held the slot, changes what
    // the work's status should say. The work itself is exempt: it is in the
    // trash, and restating a row nobody can see would only make noise.
    if entity != trash::Entity::Work {
        if let Some(work_id) = behind {
            restate(&conn, &profile_id, &work_id);
        }
    }

    Ok(entry_id)
}

/// The work a just-deleted child belonged to, read from the trash snapshot.
///
/// The live row is gone by now, so the snapshot is the only place left that
/// still knows.
fn work_behind(
    conn: &rusqlite::Connection,
    entity: trash::Entity,
    entity_id: &str,
) -> Option<String> {
    match entity {
        trash::Entity::Work => Some(entity_id.to_owned()),
        trash::Entity::Collection => None,
        _ => trash::snapshot_work_id(conn, entity, entity_id),
    }
}

#[tauri::command]
pub fn delete_version(state: State<'_, AppState>, id: String) -> Result<String> {
    discard_and_record(&state, trash::Entity::Version, &id)
}

#[tauri::command]
pub fn list_notes(state: State<'_, AppState>, filter: Option<NoteFilter>) -> Result<Vec<Note>> {
    let conn = state.conn();
    let profile_id = active_profile_id(&conn)?;
    note::list(&conn, &profile_id, &filter.unwrap_or_default())
}

#[tauri::command]
pub fn create_note(state: State<'_, AppState>, note: NewNote) -> Result<Note> {
    let mut conn = state.conn();
    let profile_id = active_profile_id(&conn)?;

    let minted = Minted::fresh();
    let logged = operation::Intent::new("note.create")
        .in_profile(&profile_id)
        .param("profile", profile_key(&conn, &profile_id)?)
        .param("note", serde_json::to_value(&note)?)
        .minted(&minted);

    recording(&mut conn, logged, |tx| {
        note::create_minted(tx, &profile_id, note, minted)
    })
}

#[tauri::command]
pub fn update_note(state: State<'_, AppState>, id: String, patch: NotePatch) -> Result<Note> {
    let mut conn = state.conn();
    let profile_id = active_profile_id(&conn)?;
    let before = note::get(&conn, &id)?;

    let at = time::now();
    let logged = operation::Intent::new("note.update")
        .in_profile(&profile_id)
        .param("profile", profile_key(&conn, &profile_id)?)
        .param("id", id.clone())
        .param("patch", serde_json::to_value(&patch)?)
        .param("before", was(before.as_ref(), &patch)?)
        .param("at", at.clone());

    recording(&mut conn, logged, |tx| note::update_at(tx, &id, patch, &at))
}

#[tauri::command]
pub fn delete_note(state: State<'_, AppState>, id: String) -> Result<String> {
    discard_and_record(&state, trash::Entity::Note, &id)
}

#[tauri::command]
pub fn dismissed_findings(state: State<'_, AppState>) -> Result<Vec<Dismissal>> {
    let conn = state.conn();
    let profile_id = active_profile_id(&conn)?;
    focus::dismissals(&conn, &profile_id)
}

#[tauri::command]
pub fn dismiss_finding(state: State<'_, AppState>, key: DismissalKey) -> Result<Dismissal> {
    let mut conn = state.conn();
    let profile_id = active_profile_id(&conn)?;

    let at = time::now();
    let logged = operation::Intent::new("finding.dismiss")
        .in_profile(&profile_id)
        .param("profile", profile_key(&conn, &profile_id)?)
        .param("key", serde_json::to_value(&key)?)
        .param("at", at.clone());

    recording(&mut conn, logged, |tx| {
        focus::dismiss_at(tx, &profile_id, &key, &at)
    })
}

#[tauri::command]
pub fn restore_finding(state: State<'_, AppState>, key: DismissalKey) -> Result<()> {
    let mut conn = state.conn();
    let profile_id = active_profile_id(&conn)?;

    let logged = operation::Intent::new("finding.restore")
        .in_profile(&profile_id)
        .param("profile", profile_key(&conn, &profile_id)?)
        .param("key", serde_json::to_value(&key)?);

    recording(&mut conn, logged, |tx| {
        focus::restore(tx, &profile_id, &key)
    })
}

#[tauri::command]
pub fn list_focus_notes(state: State<'_, AppState>) -> Result<Vec<FocusNote>> {
    let conn = state.conn();
    let profile_id = active_profile_id(&conn)?;
    focus::notes(&conn, &profile_id)
}

#[tauri::command]
pub fn create_focus_note(state: State<'_, AppState>, note: NewFocusNote) -> Result<FocusNote> {
    let mut conn = state.conn();
    let profile_id = active_profile_id(&conn)?;

    let minted = Minted::fresh();
    let logged = operation::Intent::new("focusNote.create")
        .in_profile(&profile_id)
        .param("profile", profile_key(&conn, &profile_id)?)
        .param("note", serde_json::to_value(&note)?)
        .minted(&minted);

    recording(&mut conn, logged, |tx| {
        focus::add_note_minted(tx, &profile_id, note, minted)
    })
}

#[tauri::command]
pub fn update_focus_note(
    state: State<'_, AppState>,
    id: String,
    patch: FocusNotePatch,
) -> Result<FocusNote> {
    let mut conn = state.conn();
    let profile_id = active_profile_id(&conn)?;
    let before = focus::get_note(&conn, &id)?;

    let at = time::now();
    let logged = operation::Intent::new("focusNote.update")
        .in_profile(&profile_id)
        .param("profile", profile_key(&conn, &profile_id)?)
        .param("id", id.clone())
        .param("patch", serde_json::to_value(&patch)?)
        .param("before", was(before.as_ref(), &patch)?)
        .param("at", at.clone());

    recording(&mut conn, logged, |tx| {
        focus::update_note_at(tx, &id, patch, &at)
    })
}

#[tauri::command]
pub fn reorder_focus_notes(state: State<'_, AppState>, order: Vec<String>) -> Result<()> {
    let mut conn = state.conn();
    let profile_id = active_profile_id(&conn)?;

    // The operation is written inside `reorder_notes`'s own transaction — it
    // opens one to move every note in the arrangement together. See ADR 0014.
    let logged = operation::Intent::new("focusNote.reorder")
        .in_profile(&profile_id)
        .param("profile", profile_key(&conn, &profile_id)?)
        .param("order", serde_json::to_value(&order)?);

    focus::reorder_notes(&mut conn, &profile_id, &order, Some(logged))
}

/// Rub a board note out.
///
/// It does not go through the trash the way a work or an idea does: those are
/// content, and losing one by mistake costs the person something. A board note
/// is a line on a surface — restoring it would be a drawer of crossed-out
/// reminders nobody opens.
#[tauri::command]
pub fn delete_focus_note(state: State<'_, AppState>, id: String) -> Result<()> {
    let mut conn = state.conn();
    let profile_id = active_profile_id(&conn)?;

    let logged = operation::Intent::new("focusNote.delete")
        .in_profile(&profile_id)
        .param("profile", profile_key(&conn, &profile_id)?)
        .param("id", id.clone());

    recording(&mut conn, logged, |tx| focus::delete_note(tx, &id))
}

#[tauri::command]
pub fn list_tags(state: State<'_, AppState>) -> Result<Vec<(String, i64)>> {
    let conn = state.conn();
    let profile_id = active_profile_id(&conn)?;
    note::tags(&conn, &profile_id)
}

/// Tags in use on works, for completing the next one.
///
/// Separate from `list_tags`: a note's vocabulary and a work's are different
/// vocabularies, and offering "reference" while tagging a song would be the
/// app guessing at a connection nobody made.
#[tauri::command]
pub fn work_tags(state: State<'_, AppState>) -> Result<Vec<(String, i64)>> {
    let conn = state.conn();
    let profile_id = active_profile_id(&conn)?;
    work::tags(&conn, &profile_id)
}

#[tauri::command]
pub fn score_work(state: State<'_, AppState>, work_id: String, score: NewScore) -> Result<Score> {
    let mut conn = state.conn();
    let profile_id = active_profile_id(&conn)?;

    let minted = Minted::fresh();
    let logged = operation::Intent::new("score.create")
        .in_profile(&profile_id)
        .param("profile", profile_key(&conn, &profile_id)?)
        .param("workId", work_id.clone())
        .param("score", serde_json::to_value(&score)?)
        .minted(&minted);

    let created = recording(&mut conn, logged, |tx| {
        score::create_minted(tx, &work_id, score, minted)
    })?;

    journal::record(
        &conn,
        &profile_id,
        Record::new("score.added")
            .param(
                "title",
                journal::work_title(&conn, &work_id).unwrap_or_default(),
            )
            // Rounded here rather than when the line is drawn: a total of
            // 29.999999999999993 is what floating point makes of three threes,
            // and once it is in the entry it is in there for good. The score
            // itself keeps its full precision in `work_score`; this is the
            // number a person reads.
            .param("total", (created.total * 10.0).round() / 10.0)
            .param("tier", created.tier.clone().unwrap_or_default())
            .about("work", work_id.clone()),
    );

    restate(&conn, &profile_id, &work_id);

    Ok(created)
}

#[tauri::command]
pub fn score_history(state: State<'_, AppState>, work_id: String) -> Result<Vec<Score>> {
    let conn = state.conn();
    score::history(&conn, &work_id)
}

#[tauri::command]
pub fn latest_score(state: State<'_, AppState>, work_id: String) -> Result<Option<Score>> {
    let conn = state.conn();
    score::latest(&conn, &work_id)
}

/// What each release kind makes of this work's latest answers.
///
/// Computed rather than stored: the weights live in the profile, so a verdict
/// saved beside the score would be stale the moment the craft changed its
/// mind. An unscored work has no verdicts, which is not the same as a work
/// every kind rates zero.
#[tauri::command]
pub fn kind_verdicts(
    state: State<'_, AppState>,
    work_id: String,
) -> Result<Vec<profile::config::KindVerdict>> {
    let conn = state.conn();
    let Some(latest) = score::latest(&conn, &work_id)? else {
        return Ok(Vec::new());
    };
    let profile =
        profile::active(&conn)?.ok_or_else(|| Error::Other("no profile is active".into()))?;
    Ok(profile.config.verdicts(&latest.axes))
}

#[tauri::command]
pub fn delete_score(state: State<'_, AppState>, id: String) -> Result<String> {
    discard_and_record(&state, trash::Entity::Score, &id)
}

#[tauri::command]
pub fn catalogue(state: State<'_, AppState>) -> Result<Vec<ScoredWork>> {
    let conn = state.conn();
    let profile_id = active_profile_id(&conn)?;
    score::catalogue(&conn, &profile_id)
}

#[tauri::command]
pub fn create_release(state: State<'_, AppState>, release: NewRelease) -> Result<Release> {
    let mut conn = state.conn();
    let profile_id = active_profile_id(&conn)?;

    let minted = Minted::fresh();
    let logged = operation::Intent::new("release.create")
        .in_profile(&profile_id)
        .param("profile", profile_key(&conn, &profile_id)?)
        .param("release", serde_json::to_value(&release)?)
        .minted(&minted);

    let created = recording(&mut conn, logged, |tx| {
        release::create_minted(tx, release, minted)
    })?;

    journal::record(
        &conn,
        &profile_id,
        Record::new("release.created")
            .param(
                "title",
                journal::work_title(&conn, &created.work_id).unwrap_or_default(),
            )
            .param("kind", created.kind.clone())
            .about("work", created.work_id.clone()),
    );

    Ok(created)
}

/// Edit a release: its kind, its date, the link it went out on.
///
/// Moving a date here changes the same fact `schedule_release` changes, so it
/// has to leave the same trail. Without this the calendar could quietly take a
/// work out of its slot while the work still called itself scheduled — the
/// drift v0.20 exists to prevent, reintroduced through a side door.
///
/// It does not compete for a slot the way claiming one does: this is editing a
/// booking you already hold, and a date typed into a form is not a bid.
#[tauri::command]
pub fn update_release(
    state: State<'_, AppState>,
    id: String,
    patch: ReleasePatch,
) -> Result<Release> {
    let mut conn = state.conn();
    let profile_id = active_profile_id(&conn)?;
    let before = release::get(&conn, &id)?;

    let at = time::now();
    let logged = operation::Intent::new("release.update")
        .in_profile(&profile_id)
        .param("profile", profile_key(&conn, &profile_id)?)
        .param("id", id.clone())
        .param("patch", serde_json::to_value(&patch)?)
        .param("before", was(before.as_ref(), &patch)?)
        .param("at", at.clone());

    let updated = recording(&mut conn, logged, |tx| {
        release::update_at(tx, &id, patch, &at)
    })?;

    let moved = before
        .as_ref()
        .is_some_and(|before| before.scheduled_at != updated.scheduled_at);

    if moved {
        journal::record(
            &conn,
            &profile_id,
            Record::new("release.moved")
                .param(
                    "title",
                    journal::work_title(&conn, &updated.work_id).unwrap_or_default(),
                )
                .param("slot", updated.scheduled_at.clone().unwrap_or_default())
                .about("work", updated.work_id.clone()),
        );
        restate(&conn, &profile_id, &updated.work_id);
    }

    Ok(updated)
}

#[tauri::command]
pub fn delete_release(state: State<'_, AppState>, id: String) -> Result<String> {
    discard_and_record(&state, trash::Entity::Release, &id)
}

/// Claim a calendar slot. Displacing a weaker release is reported back rather
/// than done silently — the user should see what moved.
#[tauri::command]
pub fn schedule_release(
    state: State<'_, AppState>,
    id: String,
    slot: String,
) -> Result<Scheduling> {
    let mut conn = state.conn();
    let profile_id = active_profile_id(&conn)?;

    let at = time::now();
    let logged = operation::Intent::new("release.schedule")
        .in_profile(&profile_id)
        .param("profile", profile_key(&conn, &profile_id)?)
        .param("id", id.clone())
        .param("slot", slot.clone())
        .param("at", at.clone());

    let outcome = recording(&mut conn, logged, |tx| {
        release::schedule_at(tx, &id, &slot, &at)
    })?;

    journal::record(
        &conn,
        &profile_id,
        Record::new("release.scheduled")
            .param(
                "title",
                journal::work_title(&conn, &outcome.release.work_id).unwrap_or_default(),
            )
            .param("slot", slot.clone())
            .about("work", outcome.release.work_id.clone()),
    );

    restate(&conn, &profile_id, &outcome.release.work_id);

    Ok(outcome)
}

/// Warn about every release inside the coming week that could not go out
/// today — once per release and date.
///
/// The frontend calls this at startup and after calendar changes, passing the
/// user's local date: the backend only knows UTC, which at a negative offset
/// is already tomorrow. The entry uses once-semantics — a warning the person
/// already dismissed is not re-lit by the same sweep noticing the same gap,
/// but the release moving to a new date is a new situation and warns afresh.
#[tauri::command]
pub fn warn_unready_releases(state: State<'_, AppState>, today: String) -> Result<usize> {
    let conn = state.conn();
    let profile_id = active_profile_id(&conn)?;
    record_unready_warnings(&conn, &profile_id, &today)
}

/// The body of [`warn_unready_releases`], reachable without a Tauri state.
pub fn record_unready_warnings(
    conn: &rusqlite::Connection,
    profile_id: &str,
    today: &str,
) -> Result<usize> {
    let releases = release::calendar(conn, profile_id)?;
    let unready = crate::readiness::unready_upcoming(
        &releases,
        today,
        crate::readiness::WARNING_HORIZON_DAYS,
    )?;

    for entry in &unready {
        let date = entry.release.scheduled_at.clone().unwrap_or_default();
        journal::record(
            conn,
            profile_id,
            Record::new("release.notReady")
                .param("title", entry.work_title.clone())
                .param("date", date.clone())
                .about("work", entry.release.work_id.clone())
                .once(format!("ready:{}:{date}", entry.release.id))
                .warn(),
        );
    }

    Ok(unready.len())
}

/// Where the queue would land, laid out to the profile's rhythm. Nothing is
/// written: the person approves the plan, or nothing happens.
#[tauri::command]
pub fn plan_layout(state: State<'_, AppState>, today: String) -> Result<Vec<layout::Placement>> {
    let conn = state.conn();
    let profile_id = active_profile_id(&conn)?;
    layout::plan(&conn, &profile_id, &today)
}

/// Book exactly the plan the person approved — all of it or none of it.
///
/// One journal line for the whole batch rather than one per release: fifty
/// placements telling the story fifty times is how a history stops being
/// read. Statuses are brought in line the same way the resync does — silently,
/// with the batch line standing for all of them.
#[tauri::command]
pub fn apply_layout(
    state: State<'_, AppState>,
    placements: Vec<layout::Placement>,
) -> Result<usize> {
    let mut conn = state.conn();
    let profile_id = active_profile_id(&conn)?;
    let at = time::now();
    let logged = operation::Intent::new("layout.apply")
        .in_profile(&profile_id)
        .param("profile", profile_key(&conn, &profile_id)?)
        .param("placements", serde_json::to_value(&placements)?)
        .param("at", at.clone());
    record_applied_layout(&mut conn, &profile_id, &placements, logged, &at)
}

/// The body of [`apply_layout`], reachable without a Tauri state.
///
/// The operation is written inside `layout::apply`'s own transaction — it
/// opens one to book every placement together. See ADR 0014.
pub fn record_applied_layout(
    conn: &mut rusqlite::Connection,
    profile_id: &str,
    placements: &[layout::Placement],
    logged: operation::Intent,
    at: &str,
) -> Result<usize> {
    let applied = layout::apply_at(conn, placements, Some(logged), at)?;

    if applied > 0 {
        // The plan arrives in date order, but the journal line should not
        // depend on that holding.
        let from = placements.iter().map(|p| p.date.as_str()).min();
        let to = placements.iter().map(|p| p.date.as_str()).max();
        journal::record(
            conn,
            profile_id,
            Record::new("layout.applied")
                .param("count", applied as i64)
                .param("from", from.unwrap_or_default())
                .param("to", to.unwrap_or_default()),
        );

        match profile::config_for(conn, profile_id) {
            Ok(config) => {
                if let Err(cause) = work::status::resync(conn, &config, profile_id) {
                    eprintln!("layout: could not restate the works: {cause}");
                }
            }
            Err(cause) => eprintln!("layout: could not read the profile: {cause}"),
        }
    }

    Ok(applied)
}

/// The dry run of a claim: how `schedule_release` would end, without moving
/// anything. Nothing is written and nothing is journalled — it is a look, not
/// an action.
#[tauri::command]
pub fn preview_schedule(
    state: State<'_, AppState>,
    id: String,
    slot: String,
) -> Result<release::SlotPreview> {
    let conn = state.conn();
    release::preview(&conn, &id, &slot)
}

/// Settle a date so the contest leaves it alone, or hand it back.
#[tauri::command]
pub fn set_slot_pin(state: State<'_, AppState>, id: String, pinned: bool) -> Result<Release> {
    let mut conn = state.conn();
    let profile_id = active_profile_id(&conn)?;

    let at = time::now();
    let logged = operation::Intent::new("release.setSlotPin")
        .in_profile(&profile_id)
        .param("profile", profile_key(&conn, &profile_id)?)
        .param("id", id.clone())
        .param("pinned", pinned)
        .param("at", at.clone());

    let updated = recording(&mut conn, logged, |tx| {
        release::set_slot_pin_at(tx, &id, pinned, &at)
    })?;

    // Both keys are written out literally rather than chosen inside the call.
    // The gate that checks every recorded action has a sentence reads this file
    // for literal keys, and one computed in the argument is invisible to it —
    // which means a raw key on someone's screen instead of a sentence.
    let title = journal::work_title(&conn, &updated.work_id).unwrap_or_default();
    let slot = updated.scheduled_at.clone().unwrap_or_default();
    let entry = if pinned {
        Record::new("release.pinned")
    } else {
        Record::new("release.unpinned")
    };

    journal::record(
        &conn,
        &profile_id,
        entry
            .param("title", title)
            .param("slot", slot)
            .about("work", updated.work_id.clone()),
    );

    Ok(updated)
}

#[tauri::command]
pub fn unschedule_release(state: State<'_, AppState>, id: String) -> Result<Release> {
    let mut conn = state.conn();
    let profile_id = active_profile_id(&conn)?;

    let at = time::now();
    let logged = operation::Intent::new("release.unschedule")
        .in_profile(&profile_id)
        .param("profile", profile_key(&conn, &profile_id)?)
        .param("id", id.clone())
        .param("at", at.clone());

    let release = recording(&mut conn, logged, |tx| release::unschedule_at(tx, &id, &at))?;
    restate(&conn, &profile_id, &release.work_id);
    Ok(release)
}

/// The end of the circle: something actually went out.
///
/// `at` is the day it shipped, when that is not today — a release marked late,
/// or one whose real date is known from elsewhere.
#[tauri::command]
pub fn mark_released(
    state: State<'_, AppState>,
    id: String,
    url: Option<String>,
    at: Option<String>,
) -> Result<Release> {
    let mut conn = state.conn();
    let profile_id = active_profile_id(&conn)?;

    // `at` here is the release's own day, the person's own intent — a
    // separate piece of data from the operation's timestamp below, which is
    // the moment the mark was recorded. See `release::mark_released_at`.
    let recorded_at = time::now();
    let logged = operation::Intent::new("release.markReleased")
        .in_profile(&profile_id)
        .param("profile", profile_key(&conn, &profile_id)?)
        .param("id", id.clone())
        .param("url", url.clone())
        .param("on_day", at.clone())
        .param("at", recorded_at.clone());

    let released = recording(&mut conn, logged, |tx| {
        release::mark_released_at(tx, &id, url, at, &recorded_at)
    })?;

    journal::record(
        &conn,
        &profile_id,
        Record::new("release.released")
            .param(
                "title",
                journal::work_title(&conn, &released.work_id).unwrap_or_default(),
            )
            .param("kind", released.kind.clone())
            .about("work", released.work_id.clone()),
    );

    restate(&conn, &profile_id, &released.work_id);

    Ok(released)
}

/// Undoing the mark: it did not go out after all.
///
/// The work's status follows on its own — it is derived from the facts, so
/// removing the released release is enough to send the work back to whatever it
/// was before. The link is deliberately kept; see [`release::unmark_released`].
#[tauri::command]
pub fn unmark_released(state: State<'_, AppState>, id: String) -> Result<Release> {
    let mut conn = state.conn();
    let profile_id = active_profile_id(&conn)?;

    let at = time::now();
    let logged = operation::Intent::new("release.unmarkReleased")
        .in_profile(&profile_id)
        .param("profile", profile_key(&conn, &profile_id)?)
        .param("id", id.clone())
        .param("at", at.clone());

    let planned = recording(&mut conn, logged, |tx| {
        release::unmark_released_at(tx, &id, &at)
    })?;

    journal::record(
        &conn,
        &profile_id,
        Record::new("release.unreleased")
            .param(
                "title",
                journal::work_title(&conn, &planned.work_id).unwrap_or_default(),
            )
            .param("kind", planned.kind.clone())
            .about("work", planned.work_id.clone()),
    );

    restate(&conn, &profile_id, &planned.work_id);

    Ok(planned)
}

#[tauri::command]
pub fn calendar(state: State<'_, AppState>) -> Result<Vec<ScheduledRelease>> {
    let conn = state.conn();
    let profile_id = active_profile_id(&conn)?;
    release::calendar(&conn, &profile_id)
}

#[tauri::command]
pub fn release_queue(state: State<'_, AppState>) -> Result<Vec<ScheduledRelease>> {
    let conn = state.conn();
    let profile_id = active_profile_id(&conn)?;
    release::queue(&conn, &profile_id)
}

#[tauri::command]
pub fn releases_for_work(
    state: State<'_, AppState>,
    work_id: String,
) -> Result<Vec<ScheduledRelease>> {
    let conn = state.conn();
    let profile_id = active_profile_id(&conn)?;
    release::for_work(&conn, &profile_id, &work_id)
}

#[tauri::command]
pub fn list_collections(state: State<'_, AppState>) -> Result<Vec<Collection>> {
    let conn = state.conn();
    let profile_id = active_profile_id(&conn)?;
    collection::list(&conn, &profile_id)
}

#[tauri::command]
pub fn create_collection(
    state: State<'_, AppState>,
    collection: NewCollection,
) -> Result<Collection> {
    let mut conn = state.conn();
    let profile_id = active_profile_id(&conn)?;

    let minted = Minted::fresh();
    let logged = operation::Intent::new("collection.create")
        .in_profile(&profile_id)
        .param("profile", profile_key(&conn, &profile_id)?)
        .param("collection", serde_json::to_value(&collection)?)
        .minted(&minted);

    recording(&mut conn, logged, |tx| {
        collection::create_minted(tx, &profile_id, collection, minted)
    })
}

#[tauri::command]
pub fn update_collection(
    state: State<'_, AppState>,
    id: String,
    patch: CollectionPatch,
) -> Result<Collection> {
    let mut conn = state.conn();
    let profile_id = active_profile_id(&conn)?;
    let before = collection::get(&conn, &id)?;

    let at = time::now();
    let logged = operation::Intent::new("collection.update")
        .in_profile(&profile_id)
        .param("profile", profile_key(&conn, &profile_id)?)
        .param("id", id.clone())
        .param("patch", serde_json::to_value(&patch)?)
        .param("before", was(before.as_ref(), &patch)?)
        .param("at", at.clone());

    recording(&mut conn, logged, |tx| {
        collection::update_at(tx, &id, patch, &at)
    })
}

#[tauri::command]
pub fn delete_collection(state: State<'_, AppState>, id: String) -> Result<String> {
    discard_and_record(&state, trash::Entity::Collection, &id)
}

/// Anything matching a query: works, version bodies, notes, chat messages.
///
/// One call rather than four, because the palette shows them together and
/// four round trips would arrive out of order.
#[tauri::command]
pub fn search(state: State<'_, AppState>, query: String) -> Result<Vec<Hit>> {
    let conn = state.conn();
    let profile_id = active_profile_id(&conn)?;
    search::find(&conn, &profile_id, &query)
}

/// Everything in the active profile's trash, newest first.
#[tauri::command]
pub fn list_deletions(state: State<'_, AppState>) -> Result<Vec<Deletion>> {
    let conn = state.conn();
    let profile_id = active_profile_id(&conn)?;
    trash::list(&conn, &profile_id)
}

/// Put a trashed entry back where it came from.
#[tauri::command]
pub fn restore_deletion(state: State<'_, AppState>, id: String) -> Result<()> {
    let mut conn = state.conn();
    let profile_id = active_profile_id(&conn)?;
    // What it was, read before the restore spends the entry.
    let described = trash::list(&conn, &profile_id)?
        .into_iter()
        .find(|entry| entry.id == id);

    // The operation is written inside `trash::restore`'s own transaction — see
    // its doc comment and ADR 0014.
    let logged = operation::Intent::new("trash.restore")
        .in_profile(&profile_id)
        .param("profile", profile_key(&conn, &profile_id)?)
        .param("id", id.clone());
    trash::restore(&mut conn, &id, Some(logged))?;

    if let Some(entry) = described {
        let mut record = Record::new("trash.restored").param("label", entry.label);
        let behind = work_behind(&conn, entry.entity, &entry.entity_id);
        if let Some(work_id) = behind.clone() {
            record = record.about("work", work_id);
        }
        journal::record(&conn, &profile_id, record);

        // A restored score or release is a fact again, and the status has to
        // answer for it — including for a work that came back whole.
        if let Some(work_id) = behind {
            restate(&conn, &profile_id, &work_id);
        }
    }

    Ok(())
}

/// The profile's history, newest first.
#[tauri::command]
pub fn list_journal(state: State<'_, AppState>) -> Result<Vec<Entry>> {
    let conn = state.conn();
    let profile_id = active_profile_id(&conn)?;
    journal::list(&conn, &profile_id)
}

/// One work's own history — the card's History tab.
#[tauri::command]
pub fn journal_for_work(state: State<'_, AppState>, work_id: String) -> Result<Vec<Entry>> {
    let conn = state.conn();
    journal::for_entity(&conn, "work", &work_id)
}

/// How many entries are asking to be looked at.
#[tauri::command]
pub fn unread_journal(state: State<'_, AppState>) -> Result<i64> {
    let conn = state.conn();
    let profile_id = active_profile_id(&conn)?;
    journal::unread_count(&conn, &profile_id)
}

/// Mark everything currently unread as seen. Returns how many were.
#[tauri::command]
pub fn mark_journal_read(state: State<'_, AppState>) -> Result<usize> {
    let conn = state.conn();
    let profile_id = active_profile_id(&conn)?;
    journal::mark_read(&conn, &profile_id)
}

/// Drop one entry for good.
#[tauri::command]
pub fn purge_deletion(state: State<'_, AppState>, id: String) -> Result<()> {
    let mut conn = state.conn();
    let profile_id = active_profile_id(&conn)?;

    // The operation is written inside `trash::purge`'s own transaction — see
    // its doc comment and ADR 0014.
    let logged = operation::Intent::new("trash.purge")
        .in_profile(&profile_id)
        .param("profile", profile_key(&conn, &profile_id)?)
        .param("id", id.clone());
    trash::purge(&mut conn, &id, Some(logged))
}

/// Empty the active profile's trash. Returns how many entries went.
#[tauri::command]
pub fn empty_trash(state: State<'_, AppState>) -> Result<usize> {
    let mut conn = state.conn();
    let profile_id = active_profile_id(&conn)?;

    let logged = operation::Intent::new("trash.empty")
        .in_profile(&profile_id)
        .param("profile", profile_key(&conn, &profile_id)?);

    recording(&mut conn, logged, |tx| trash::empty(tx, &profile_id))
}

/// What pressing undo would take back, if anything.
///
/// Read fresh rather than remembered by the frontend: between the last action
/// and the keystroke, a sweep or a second window may have written, and an offer
/// built from a stale memory would name the wrong thing.
#[tauri::command]
pub fn last_undoable(state: State<'_, AppState>) -> Result<Option<undo::Undoable>> {
    let conn = state.conn();
    undo::last(&conn)
}

/// Take back the operation the offer names.
///
/// The id travels back rather than being implied, so that an offer read a
/// moment ago cannot silently reverse something newer — `undo::undo` refuses
/// when they disagree.
#[tauri::command]
pub fn undo_last(state: State<'_, AppState>, operation: String) -> Result<undo::Undoable> {
    let mut conn = state.conn();
    let taken = undo::undo(&mut conn, &operation)?;

    if let Ok(profile_id) = active_profile_id(&conn) {
        journal::record(
            &conn,
            &profile_id,
            Record::new("undo.done").param("action", taken.action.clone()),
        );
    }

    Ok(taken)
}

/// Write the active profile out as markdown.
#[tauri::command]
pub fn export_markdown(state: State<'_, AppState>, directory: String) -> Result<ExportReport> {
    let conn = state.conn();
    export::to_markdown(&conn, std::path::Path::new(&directory))
}

/// Copy the workspace somewhere safe.
#[tauri::command]
pub fn backup_workspace(state: State<'_, AppState>, destination: String) -> Result<String> {
    let conn = state.conn();
    let written = backup::write(&conn, std::path::Path::new(&destination))?;
    Ok(written.display().to_string())
}

/// Suggested file name for a backup taken now.
#[tauri::command]
pub fn suggested_backup_name() -> String {
    backup::suggested_name(&crate::time::now())
}

/// Where the workspace file lives, so the user can find or replace it.
#[tauri::command]
pub fn workspace_path(state: State<'_, AppState>) -> String {
    state.path().display().to_string()
}

/// Bring in a slice of a predecessor workspace. Existing titles are skipped.
#[tauri::command]
pub fn import_legacy(state: State<'_, AppState>, source: String) -> Result<ImportReport> {
    let mut conn = state.conn();
    let profile_id = active_profile_id(&conn)?;
    import::from_legacy(&mut conn, std::path::Path::new(&source), &profile_id)
}

/// Everything installed under the plugin naming convention, usable or not.
#[tauri::command]
pub fn list_plugins(state: State<'_, AppState>) -> Vec<Plugin> {
    let directory = state
        .path()
        .parent()
        .map(std::path::Path::to_path_buf)
        .unwrap_or_default();
    plugin::discover(&directory)
}

/// Run a plugin command against a release or a work, and merge whatever it
/// returns into that row's `meta`.
///
/// A plugin can add and overwrite its own keys but cannot clear the rest —
/// losing unrelated metadata to a third-party integration is not recoverable.
#[tauri::command]
pub fn run_plugin(
    state: State<'_, AppState>,
    executable: String,
    command: String,
    target: Target,
    id: String,
) -> Result<Option<String>> {
    let directory = state
        .path()
        .parent()
        .map(std::path::Path::to_path_buf)
        .unwrap_or_default();

    let found = plugin::discover(&directory)
        .into_iter()
        .find(|candidate| candidate.executable == executable)
        .ok_or_else(|| Error::not_found("plugin", executable.clone()))?;

    if !found.usable {
        return Err(Error::Other(
            found
                .reason
                .unwrap_or_else(|| format!("`{executable}` cannot be used")),
        ));
    }

    let conn = state.conn();
    let subject = match target {
        Target::Release => serde_json::to_value(
            release::get(&conn, &id)?.ok_or_else(|| Error::not_found("release", id.clone()))?,
        )?,
        Target::Work => {
            let found =
                work::get(&conn, &id)?.ok_or_else(|| Error::not_found("work", id.clone()))?;
            let mut value = serde_json::to_value(&found)?;

            // A plugin acting on a work almost always wants its text. Sending
            // only the row would make every plugin ask for the body back.
            let mut bodies = serde_json::Map::new();
            for summary in version::list(&conn, &id)? {
                if bodies.contains_key(&summary.role) {
                    continue;
                }
                if let Some(full) = version::get(&conn, &summary.id)? {
                    bodies.insert(summary.role.clone(), serde_json::Value::String(full.body));
                }
            }
            if let Some(object) = value.as_object_mut() {
                object.insert("bodies".into(), serde_json::Value::Object(bodies));
            }
            value
        }
    };
    drop(conn);

    let outcome = plugin::invoke(
        std::path::Path::new(&found.path),
        &plugin::Invocation {
            command: &command,
            target,
            subject: subject.clone(),
        },
    )?;

    if !outcome.meta.is_empty() {
        let conn = state.conn();
        let existing = subject
            .get("meta")
            .and_then(serde_json::Value::as_object)
            .cloned()
            .unwrap_or_default();
        let merged = plugin::merge_meta(&existing, &outcome.meta);

        match target {
            Target::Release => {
                release::update(
                    &conn,
                    &id,
                    ReleasePatch {
                        meta: Some(merged),
                        ..Default::default()
                    },
                )?;
            }
            Target::Work => {
                work::update(
                    &conn,
                    &id,
                    WorkPatch {
                        meta: Some(merged),
                        ..Default::default()
                    },
                )?;
            }
        }
    }

    Ok(outcome.message)
}

/// Whether the AI panel can work on this machine. Never an error: "not
/// installed" is something the panel shows, and the rest of kilna is unaffected.
#[tauri::command]
pub fn assistant_status() -> cli::Availability {
    cli::probe()
}

/// Chats of the active profile as the list draws them — named, priced, tied
/// to their work. `work_id` narrows to one work's chats.
#[tauri::command]
pub fn list_chat_summaries(
    state: State<'_, AppState>,
    work_id: Option<String>,
) -> Result<Vec<assistant::ChatSummary>> {
    let conn = state.conn();
    let profile_id = active_profile_id(&conn)?;
    assistant::summaries(&conn, &profile_id, work_id.as_deref())
}

#[tauri::command]
pub fn create_chat(state: State<'_, AppState>, chat: NewChat) -> Result<Chat> {
    let conn = state.conn();
    let profile_id = active_profile_id(&conn)?;
    assistant::create(&conn, &profile_id, chat)
}

/// Name a chat, or clear the name with `None` so it borrows its first
/// question again.
#[tauri::command]
pub fn rename_chat(state: State<'_, AppState>, id: String, title: Option<String>) -> Result<()> {
    let conn = state.conn();
    assistant::rename(&conn, &id, title.as_deref())
}

#[tauri::command]
pub fn get_transcript(state: State<'_, AppState>, chat_id: String) -> Result<Option<Transcript>> {
    let conn = state.conn();
    assistant::transcript(&conn, &chat_id)
}

#[tauri::command]
pub fn delete_chat(state: State<'_, AppState>, id: String) -> Result<()> {
    let conn = state.conn();
    assistant::delete(&conn, &id)
}

/// Send a prompt and wait for the reply.
///
/// This blocks for as long as the CLI takes. Kept for callers that want one
/// answer and nothing else; the panel uses [`start_run`], which returns at once
/// and reports the rest as events.
#[tauri::command]
pub fn ask_assistant(
    state: State<'_, AppState>,
    chat_id: String,
    prompt: String,
) -> Result<Message> {
    let workdir = state.assistant_dir();
    let mut conn = state.conn();
    assistant::ask(&mut conn, &chat_id, &prompt, workdir.as_deref())
}

/// Sends run events to the window.
///
/// A failed emit is not worth failing a run over: the panel replays from the
/// stored events whenever it comes back.
struct WindowSink(AppHandle);

impl Sink for WindowSink {
    fn emit(&self, emission: &Emission) {
        let _ = self.0.emit(assistant_run::EVENT, emission);
    }
}

/// Start a run and return before the CLI answers.
///
/// The run belongs to the chat from here on: navigating away, closing the
/// panel or opening another work leaves it going. What it says arrives as
/// `assistant:run` events, and is stored as it arrives so a panel that was
/// elsewhere can replay it.
#[tauri::command]
pub fn start_run(
    app: AppHandle,
    state: State<'_, AppState>,
    chat_id: String,
    prompt: String,
) -> Result<Run> {
    let runs = Arc::clone(state.runs());
    let workdir = state.assistant_dir();

    let (run, stream) = {
        let conn = state.conn();
        assistant_run::start(&conn, &runs, &chat_id, &prompt, workdir.as_deref())?
    };

    let sink: Arc<dyn Sink> = Arc::new(WindowSink(app.clone()));
    let started = run.clone();

    // A thread rather than an async task: the CLI is read with blocking IO, and
    // a run holds its thread for as long as the answer takes.
    std::thread::spawn(move || {
        let open = || app.state::<AppState>().inner().open_alongside();
        assistant_run::pump(&runs, &sink, &started, &stream, open);
    });

    Ok(run)
}

/// What a started task tells the card: where it went, and what it is.
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StartedTask {
    pub chat_id: String,
    pub run_id: String,
    /// The key the card disables its button by.
    pub task_key: String,
    /// The chat's name, so a toast can say where the answer will be.
    pub title: String,
}

/// Run a profile action against a work without opening the panel.
///
/// The action is rendered, given a chat of its own, and started — the call
/// returns as soon as the CLI is spawned, like [`start_run`]. Nothing is
/// inserted anywhere by the run itself: the answer lands in its chat, and what
/// the card does with it is the card's decision.
#[tauri::command]
pub fn start_task(
    app: AppHandle,
    state: State<'_, AppState>,
    work_id: String,
    action: String,
) -> Result<StartedTask> {
    spawn_task(&app, state.inner(), &work_id, &action)
}

/// Start one task and put a thread on it.
///
/// Shared by the single-task command and by whatever picks the queue up, so
/// that a task started third in line is started exactly the way a clicked one
/// is. The thread it spawns outlives the call: when the run ends, it looks for
/// the next waiting task itself.
fn spawn_task(
    app: &AppHandle,
    state: &AppState,
    work_id: &str,
    action: &str,
) -> Result<StartedTask> {
    let runs = Arc::clone(state.runs());
    let workdir = state.assistant_dir();

    // Checked before the chat is opened: a refused duplicate must not leave an
    // empty chat behind for every impatient second click.
    let key = assistant::task::key(action, work_id);
    if runs.task_running(&key) {
        return Err(crate::error::Error::Assistant(
            "This is already running. Wait for it to finish.".into(),
        ));
    }

    let (prepared, run, stream) = {
        let conn = state.conn();
        let prepared = assistant::task::prepare(&conn, work_id, action)?;
        let (run, stream) = assistant_run::start_as(
            &conn,
            &runs,
            &prepared.chat_id,
            &prepared.prompt,
            workdir.as_deref(),
            Some(prepared.key.clone()),
        )?;
        (prepared, run, stream)
    };

    let sink: Arc<dyn Sink> = Arc::new(WindowSink(app.clone()));
    let started = run.clone();
    let app = app.clone();

    std::thread::spawn(move || {
        {
            let handle = app.clone();
            let open = move || handle.state::<AppState>().inner().open_alongside();
            assistant_run::pump(&runs, &sink, &started, &stream, open);
        }

        // The slot this run was holding is free as of `pump` returning, and
        // the queue is drained here rather than on a timer because this is the
        // only moment anything is known to have changed.
        drain_queue(&app);
    });

    Ok(StartedTask {
        chat_id: prepared.chat_id,
        run_id: run.id,
        task_key: prepared.key,
        title: prepared.title,
    })
}

/// Start waiting tasks until the slots are full or nothing is left.
///
/// A task that cannot start is dropped rather than put back: the reasons it
/// fails here — the work is gone, the profile no longer has the action — do not
/// get better by waiting, and a queue that retries them forever would spawn a
/// process per attempt. What is lost is a task nobody could have run; the
/// journal keeps the record.
fn drain_queue(app: &AppHandle) {
    loop {
        let state = app.state::<AppState>();
        let state = state.inner();

        if !state.runs().has_slot() {
            return;
        }

        let Some(next) = state.queue().pop() else {
            return;
        };

        // A task that cannot start is dropped rather than put back: it takes
        // its own thread with it, and that thread drains again when it ends.
        // Only a failure keeps this loop going, and only to reach the next
        // task that might work.
        let outcome = spawn_task(app, state, &next.work_id, &next.action);
        let _ = app.emit(TASK_QUEUE_EVENT, queue_state(state));

        match outcome {
            Ok(_) => return,
            Err(cause) => eprintln!("assistant: a queued task could not start: {cause}"),
        }
    }
}

/// The event carrying how much of a batch is left.
pub const TASK_QUEUE_EVENT: &str = "assistant:queue";

/// What is waiting and what is going, as one answer.
///
/// Both halves travel together because a button asks one question — "is this
/// action busy?" — and a running task and a queued one are both a yes.
#[derive(serde::Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TaskQueue {
    /// Keys of tasks with a process alive.
    pub running: Vec<String>,
    /// Keys of tasks waiting for a slot.
    pub waiting: Vec<String>,
}

fn queue_state(state: &AppState) -> TaskQueue {
    TaskQueue {
        running: state.runs().active_tasks(),
        waiting: state.queue().keys(),
    }
}

/// Chats holding a question a background task asked, oldest first.
///
/// Asked from every screen, because that is the point: a task was left alone,
/// and the question it came back with has to be findable from wherever the
/// person happens to be.
#[tauri::command]
pub fn waiting_chats(state: State<'_, AppState>) -> Result<Vec<assistant::ChatSummary>> {
    let conn = state.conn();
    let profile_id = active_profile_id(&conn)?;
    assistant::waiting(&conn, &profile_id)
}

/// Clear a chat's question — answered, or dismissed by hand.
#[tauri::command]
pub fn clear_waiting(state: State<'_, AppState>, chat_id: String) -> Result<()> {
    let conn = state.conn();
    assistant::clear_waiting(&conn, &chat_id)
}

/// Which profile actions are running right now, as task keys.
///
/// A card asks on mount: a run started before this screen existed still owns
/// its button.
#[tauri::command]
pub fn active_tasks(state: State<'_, AppState>) -> Vec<String> {
    let mut keys = state.runs().active_tasks();
    // A queued task owns its button exactly as a running one does: from the
    // card's side, "already asked for" is the whole question.
    keys.extend(state.queue().keys());
    keys.sort();
    keys.dedup();
    keys
}

/// Run one profile action against many works.
///
/// The slots are filled and the rest queued, because the alternative — three
/// runs and a row of "the machine is busy" errors — is the feature not
/// working. Returns what the batch became so the catalogue can say it in one
/// sentence rather than one toast per work.
#[tauri::command]
pub fn start_tasks(
    app: AppHandle,
    state: State<'_, AppState>,
    work_ids: Vec<String>,
    action: String,
) -> Result<StartedBatch> {
    let inner = state.inner();

    // Refused before anything is started rather than once per work: an action
    // the profile does not have is a mistake about the whole batch. The label
    // comes from the same read — the feed is written for a person, and
    // `critique` is not what the button said.
    let label = {
        let conn = inner.conn();
        let profile =
            profile::active(&conn)?.ok_or_else(|| Error::Other("no profile is active".into()))?;
        profile
            .config
            .prompts
            .iter()
            .find(|prompt| prompt.key == action)
            .ok_or_else(|| Error::not_found("prompt", &action))?
            .label
            .clone()
    };

    let mut started = 0usize;
    let mut queued = 0usize;
    let mut skipped = 0usize;

    for work_id in &work_ids {
        let key = assistant::task::key(&action, work_id);

        // Already asked for, whether it is running or waiting. Counted rather
        // than raised: in a batch, "that one was already going" is not an
        // error, it is the reason the number is smaller.
        if inner.runs().task_running(&key) || inner.queue().holds(&key) {
            skipped += 1;
            continue;
        }

        if inner.runs().has_slot() {
            match spawn_task(&app, inner, work_id, &action) {
                Ok(_) => started += 1,
                // One work failing must not take the batch with it: the others
                // are unrelated, and a half-run batch is more useful than none.
                Err(cause) => {
                    eprintln!("assistant: {work_id} could not start: {cause}");
                    skipped += 1;
                }
            }
        } else if inner.queue().push(assistant::queue::Pending {
            work_id: work_id.clone(),
            action: action.clone(),
        }) {
            queued += 1;
        } else {
            skipped += 1;
        }
    }

    if started + queued > 0 {
        let conn = inner.conn();
        let profile_id = active_profile_id(&conn)?;
        journal::record(
            &conn,
            &profile_id,
            Record::new("assistant.batchStarted")
                .param("action", label)
                .param("count", i64::try_from(started + queued).unwrap_or(i64::MAX)),
        );
    }

    let _ = app.emit(TASK_QUEUE_EVENT, queue_state(inner));

    Ok(StartedBatch {
        started,
        queued,
        skipped,
    })
}

/// What a batch became.
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StartedBatch {
    /// Works whose run is already going.
    pub started: usize,
    /// Works waiting for a slot.
    pub queued: usize,
    /// Works passed over — already asked for, or unable to start.
    pub skipped: usize,
}

/// What is running and what is waiting.
#[tauri::command]
pub fn task_queue(state: State<'_, AppState>) -> TaskQueue {
    queue_state(state.inner())
}

/// Drop everything still waiting, leaving the runs already going alone.
///
/// The two halves are separate on purpose: a person who queued forty works and
/// changed their mind wants the forty stopped, not the three already talking to
/// the CLI killed mid-answer. Those can be cancelled one by one, and their
/// answers are already partly paid for.
#[tauri::command]
pub fn clear_task_queue(app: AppHandle, state: State<'_, AppState>) -> usize {
    let dropped = state.queue().clear();
    let _ = app.emit(TASK_QUEUE_EVENT, queue_state(state.inner()));
    dropped
}

/// Stop a run. Whatever it had already said stays in the chat.
#[tauri::command]
pub fn cancel_run(state: State<'_, AppState>, id: String) -> Result<()> {
    if !state.runs().cancel(&id) {
        // Already finished between the click and the call — nothing to stop,
        // and nothing worth showing a person.
        return Ok(());
    }
    Ok(())
}

/// Runs of a chat, newest first — the panel's replay.
#[tauri::command]
pub fn list_runs(state: State<'_, AppState>, chat_id: String) -> Result<Vec<Run>> {
    let conn = state.conn();
    assistant_run::list(&conn, &chat_id)
}

/// Which chats are working right now, for the panel's badge.
#[tauri::command]
pub fn active_runs(state: State<'_, AppState>) -> Vec<String> {
    state.runs().active_chats()
}

/// Fill a profile prompt template with a work's details.
#[tauri::command]
pub fn render_prompt(
    state: State<'_, AppState>,
    work_id: String,
    template: String,
) -> Result<String> {
    let conn = state.conn();
    prompt::for_work(&conn, &work_id, &template)
}

#[tauri::command]
pub fn set_collection_contents(
    state: State<'_, AppState>,
    id: String,
    work_ids: Vec<String>,
) -> Result<()> {
    let mut conn = state.conn();
    let profile_id = active_profile_id(&conn)?;

    let at = time::now();
    let logged = operation::Intent::new("collection.setContents")
        .in_profile(&profile_id)
        .param("profile", profile_key(&conn, &profile_id)?)
        .param("id", id.clone())
        .param("workIds", serde_json::to_value(&work_ids)?)
        .param("at", at.clone());

    // The operation is written inside `set_contents_at`'s own transaction — it
    // opens one to move every work in the new contents together. See ADR 0014.
    collection::set_contents_at(&mut conn, &id, &work_ids, &at, Some(logged))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{db, journal, layout, profile, release, work};

    /// The batch speaks once: applying a layout writes one line with the count
    /// and the range, not one per release — and the statuses still catch up,
    /// silently, the way the resync does it.
    #[test]
    fn an_applied_layout_is_one_journal_line_and_the_statuses_follow() {
        let mut conn = db::open_in_memory().unwrap();
        profile::seed(&conn).unwrap();
        let profile_id = profile::active(&conn).unwrap().unwrap().id;

        let mut work_ids = Vec::new();
        for title in ["First", "Second"] {
            let work = work::create(
                &conn,
                &profile_id,
                work::NewWork {
                    kind: "song".into(),
                    title: title.into(),
                    ..NewWork::default()
                },
            )
            .unwrap();
            crate::score::create(
                &conn,
                &work.id,
                crate::score::NewScore {
                    axes: serde_json::json!({ "hook": 7.0 })
                        .as_object()
                        .cloned()
                        .unwrap(),
                    version_id: None,
                    note: None,
                    rater: None,
                },
            )
            .unwrap();
            release::create(
                &conn,
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
            work_ids.push(work.id);
        }

        let plan = layout::plan(&conn, &profile_id, "2026-09-01").unwrap();
        let logged = operation::Intent::new("layout.apply").in_profile(&profile_id);
        record_applied_layout(&mut conn, &profile_id, &plan, logged, &time::now()).unwrap();

        let entries = journal::list(&conn, &profile_id).unwrap();
        let batch: Vec<_> = entries
            .iter()
            .filter(|entry| entry.action == "layout.applied")
            .collect();
        assert_eq!(batch.len(), 1);
        assert_eq!(batch[0].params["count"], 2);
        assert_eq!(
            entries
                .iter()
                .filter(|entry| entry.action == "work.restated")
                .count(),
            0,
            "the batch line stands for the status changes"
        );

        for work_id in &work_ids {
            let restated = work::get(&conn, work_id).unwrap().unwrap();
            assert_eq!(restated.status, "scheduled");
        }
    }

    /// The whole warning path: an unready release inside the window writes one
    /// warning, and the same sweep running again — every startup does — leaves
    /// it exactly as the person left it.
    #[test]
    fn unready_warnings_are_written_once_and_stay_dismissed() {
        let conn = db::open_in_memory().unwrap();
        profile::seed(&conn).unwrap();
        let profile_id = profile::active(&conn).unwrap().unwrap().id;

        let work = work::create(
            &conn,
            &profile_id,
            work::NewWork {
                kind: "song".into(),
                title: "Subject".into(),
                ..NewWork::default()
            },
        )
        .unwrap();
        release::create(
            &conn,
            release::NewRelease {
                work_id: work.id,
                kind: "clip".into(),
                title: None,
                scheduled_at: Some("2026-09-03".into()),
                meta: None,
                scheduled_time: None,
                time_zone: None,
            },
        )
        .unwrap();

        assert_eq!(
            record_unready_warnings(&conn, &profile_id, "2026-09-01").unwrap(),
            1
        );
        let entries = journal::list(&conn, &profile_id).unwrap();
        let warning = entries
            .iter()
            .find(|entry| entry.action == "release.notReady")
            .expect("the gap inside the window is worth a line");
        assert_eq!(warning.params["title"], "Subject");
        assert_eq!(warning.params["date"], "2026-09-03");
        assert_eq!(journal::unread_count(&conn, &profile_id).unwrap(), 1);

        journal::mark_read(&conn, &profile_id).unwrap();
        record_unready_warnings(&conn, &profile_id, "2026-09-01").unwrap();

        let entries = journal::list(&conn, &profile_id).unwrap();
        assert_eq!(
            entries
                .iter()
                .filter(|entry| entry.action == "release.notReady")
                .count(),
            1,
            "the same standing gap is one line, not one per startup"
        );
        assert_eq!(
            journal::unread_count(&conn, &profile_id).unwrap(),
            0,
            "a dismissed warning stays dismissed"
        );
    }
}
