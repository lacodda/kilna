use std::path::PathBuf;
use std::sync::Arc;

use tauri::{AppHandle, Emitter, Manager, State};

use crate::asset;
use crate::assistant::run::{self as assistant_run, Emission, Run, Sink};
use crate::assistant::{self, Chat, Message, NewChat, Transcript, cli, prompt};
use crate::collection::{self, Collection, CollectionPatch, NewCollection};
use crate::comment::{self, Comment, CommentFilter, CommentPatch, NewComment};
use crate::cut;
use crate::error::{Error, Result};
use crate::exchange::backup;
use crate::exchange::export::{self, ExportReport};
use crate::exchange::import::{self, ImportReport};
use crate::exchange::package;
use crate::focus::{self, Dismissal, DismissalKey, FocusNote, FocusNotePatch, NewFocusNote};
use crate::journal::{self, Entry, Record};
use crate::layout;
use crate::link::{self, Links, NewLink};
use crate::minted::Minted;
use crate::note::{self, NewNote, Note, NoteFilter, NotePatch};
use crate::operation;
use crate::plugin::{self, manifest::Plugin, manifest::Target};
use crate::profile::config::Label;
use crate::profile::{self, Profile, Workspace};
use crate::release::{self, NewRelease, Release, ReleasePatch, ScheduledRelease, Scheduling};
use crate::release_meta;
use crate::reversal;
use crate::scene::{self, NewScene, Scene, ScenePatch};
use crate::scene_frame;
use crate::scene_note::{self, SceneNote};
use crate::score::{self, NewScore, Score, ScoredWork};
use crate::search::{self, Hit};
use crate::state::AppState;
use crate::style_brick;
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
///
/// Logged as `profile.update` with the whole document before and after: the
/// vocabulary is part of the workspace a replay rebuilds - statuses, axes and
/// kinds are read through it - and it used to be written past the log, so a
/// rebuilt workspace came back with the profile as it was first seeded.
#[tauri::command]
pub fn update_profile_config(
    state: State<'_, AppState>,
    id: String,
    config: profile::config::ProfileConfig,
) -> Result<Profile> {
    let mut conn = state.conn();
    let before = profile::config_for(&conn, &id)?;

    let at = time::now();
    let logged = operation::Intent::new("profile.update")
        .in_profile(&id)
        .param("profile", profile_key(&conn, &id)?)
        .param("id", id.clone())
        .param("config", serde_json::to_value(&config)?)
        .param("before", serde_json::to_value(&before)?)
        .param("at", at.clone());

    recording(&mut conn, logged, |tx| {
        profile::update_config_at(tx, &id, &config, &at)
    })
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
pub(crate) fn recording<T>(
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
pub(crate) fn was<T: serde::Serialize, P: serde::Serialize>(
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
pub(crate) fn restate(conn: &rusqlite::Connection, profile_id: &str, work_id: &str) {
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
    // Any kind's word will do here: the batch may hold songs and videos, and
    // a work whose kind lacks the word is skipped below, not refused.
    let config = profile::config_for(&conn, &profile_id)?;
    if !config
        .all_statuses()
        .iter()
        .any(|known| known.key == status)
    {
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
pub fn update_version_body(
    state: State<'_, AppState>,
    id: String,
    body: String,
) -> Result<Version> {
    let mut conn = state.conn();
    let profile_id = active_profile_id(&conn)?;

    // The body before, whole, so an undo has something to put back. Not a
    // diff: a body is one field, and a diff would be a second way of storing
    // text (ADR 0002). Nothing is journaled here — the session's edits are one
    // change to the person, and the version's creation already made its line.
    let before = version::get(&conn, &id)?
        .map(|found| found.body)
        .unwrap_or_default();

    let at = time::now();
    let logged = operation::Intent::new("version.edit")
        .in_profile(&profile_id)
        .param("profile", profile_key(&conn, &profile_id)?)
        .param("id", id.clone())
        .param("body", body.clone())
        .param("before", before)
        .param("at", at.clone());

    recording(&mut conn, logged, |tx| {
        version::update_body_at(tx, &id, &body, &at)
    })
}

#[tauri::command]
pub fn set_current_version(
    state: State<'_, AppState>,
    work_id: String,
    version_id: String,
) -> Result<()> {
    let mut conn = state.conn();
    let profile_id = active_profile_id(&conn)?;
    // Refused before anything is written or logged: another work's text is
    // not this work's version.
    version::check_belongs(&conn, &work_id, &version_id)?;
    let before = work::get(&conn, &work_id)?;

    // A work.update of the one field, like any other edit of a work. Written
    // past the log until v0.76.1: Ctrl+Z after "make current" took back the
    // edit before it, and a replay rebuilt the old current version.
    let patch = WorkPatch {
        current_version_id: Some(Some(version_id)),
        ..WorkPatch::default()
    };
    let at = time::now();
    let logged = operation::Intent::new("work.update")
        .in_profile(&profile_id)
        .param("profile", profile_key(&conn, &profile_id)?)
        .param("id", work_id.clone())
        .param("patch", serde_json::to_value(&patch)?)
        .param("before", was(before.as_ref(), &patch)?)
        .param("at", at.clone());

    recording(&mut conn, logged, |tx| {
        work::update_at(tx, &work_id, patch, &at)
    })?;
    Ok(())
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
        // Neither hangs off a work: a collection holds works, and a style is
        // the workspace's own dictionary.
        trash::Entity::Collection | trash::Entity::Style => None,
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

/// Turn a note into a work, its body the work's first version.
///
/// One operation for the three rows it touches — the work, its version, the
/// note moved to the trash — so an undo takes the whole gesture back rather
/// than restoring the note beside a work that still holds its text.
#[tauri::command]
pub fn promote_note(
    state: State<'_, AppState>,
    id: String,
    promotion: note::Promotion,
) -> Result<note::Promoted> {
    let mut conn = state.conn();
    let profile_id = active_profile_id(&conn)?;

    let ids = note::PromotionIds::fresh();
    let logged = operation::Intent::new("note.promote")
        .in_profile(&profile_id)
        .param("profile", profile_key(&conn, &profile_id)?)
        .param("id", id.clone())
        .param("promotion", serde_json::to_value(&promotion)?)
        .param("workId", ids.work.id().to_owned())
        .param("versionId", ids.version.id().to_owned())
        .param("entryId", ids.deletion.id().to_owned())
        .param("title", promotion.title.trim().to_owned())
        .param("at", ids.work.at().to_owned());

    let promoted = recording(&mut conn, logged, |tx| {
        note::promote_in(tx, &profile_id, &id, promotion, &ids)
    })?;

    journal::record(
        &conn,
        &profile_id,
        Record::new("note.promoted")
            .param(
                "title",
                journal::work_title(&conn, &promoted.work_id).unwrap_or_default(),
            )
            .about("work", promoted.work_id.clone()),
    );

    Ok(promoted)
}

#[tauri::command]
pub fn list_comments(
    state: State<'_, AppState>,
    filter: Option<CommentFilter>,
) -> Result<Vec<Comment>> {
    let conn = state.conn();
    let profile_id = active_profile_id(&conn)?;
    comment::list(&conn, &profile_id, &filter.unwrap_or_default())
}

/// Every channel comments came through, with how many still wait on each.
#[tauri::command]
pub fn comment_channels(state: State<'_, AppState>) -> Result<Vec<(String, i64)>> {
    let conn = state.conn();
    let profile_id = active_profile_id(&conn)?;
    comment::channels(&conn, &profile_id)
}

/// How many comments a work has, and how many of them wait: its tab's counter.
#[derive(serde::Serialize)]
pub struct CommentCount {
    pub total: i64,
    pub waiting: i64,
}

#[tauri::command]
pub fn count_work_comments(state: State<'_, AppState>, work_id: String) -> Result<CommentCount> {
    let conn = state.conn();
    let (total, waiting) = comment::count_for_work(&conn, &work_id)?;
    Ok(CommentCount { total, waiting })
}

#[tauri::command]
pub fn create_comment(state: State<'_, AppState>, comment: NewComment) -> Result<Comment> {
    let mut conn = state.conn();
    let profile_id = active_profile_id(&conn)?;

    let minted = Minted::fresh();
    let logged = operation::Intent::new("comment.create")
        .in_profile(&profile_id)
        .param("profile", profile_key(&conn, &profile_id)?)
        .param("comment", serde_json::to_value(&comment)?)
        .minted(&minted);

    recording(&mut conn, logged, |tx| {
        comment::create_minted(tx, &profile_id, comment, minted)
    })
}

#[tauri::command]
pub fn update_comment(
    state: State<'_, AppState>,
    id: String,
    patch: CommentPatch,
) -> Result<Comment> {
    let mut conn = state.conn();
    let profile_id = active_profile_id(&conn)?;
    let before = comment::get(&conn, &id)?;

    let at = time::now();
    let logged = operation::Intent::new("comment.update")
        .in_profile(&profile_id)
        .param("profile", profile_key(&conn, &profile_id)?)
        .param("id", id.clone())
        .param("patch", serde_json::to_value(&patch)?)
        .param("before", was(before.as_ref(), &patch)?)
        .param("at", at.clone());

    recording(&mut conn, logged, |tx| {
        comment::update_at(tx, &id, patch, &at)
    })
}

#[tauri::command]
pub fn delete_comment(state: State<'_, AppState>, id: String) -> Result<String> {
    discard_and_record(&state, trash::Entity::Comment, &id)
}

/// What a `[[work:id]]` or `[[version:id]]` link points at.
///
/// One row per link that resolves: the title to draw, and for a version the
/// work whose card it opens. A link to something deleted is simply absent, and
/// the window draws it as the text it was — a dead link is worse than prose.
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolvedLink {
    pub id: String,
    /// `work` or `version`, echoed so the window can match the row to the link
    /// without assuming an order.
    pub target: String,
    pub title: String,
    /// The work whose card opens. The work's own id for a work.
    pub work_id: String,
}

/// Resolve the links in a body, in one round trip.
///
/// One call for the whole body rather than one per link: a note with twenty
/// references would otherwise be twenty queries drawn one frame apart, which
/// is the shape that made the predecessor's screens flicker.
#[tauri::command]
pub fn resolve_links(
    state: State<'_, AppState>,
    works: Vec<String>,
    versions: Vec<String>,
) -> Result<Vec<ResolvedLink>> {
    let conn = state.conn();
    let mut out = Vec::with_capacity(works.len() + versions.len());

    for id in works {
        let Some(work) = work::get(&conn, &id)? else {
            continue;
        };
        out.push(ResolvedLink {
            id,
            target: "work".into(),
            title: work.title,
            work_id: work.id,
        });
    }

    for id in versions {
        let Some(found) = version::get(&conn, &id)? else {
            continue;
        };
        // The label when it has one, else the role and revision: "lyrics 3"
        // says more in a sentence than a uuid ever will.
        let title = found
            .label
            .clone()
            .filter(|label| !label.trim().is_empty())
            .unwrap_or_else(|| format!("{} {}", found.role, found.revision));
        out.push(ResolvedLink {
            id,
            target: "version".into(),
            title,
            work_id: found.work_id,
        });
    }

    Ok(out)
}

/// The workspace's style dictionary, in the profile's order of types.
#[tauri::command]
pub fn list_style_bricks(
    state: State<'_, AppState>,
    filter: Option<style_brick::StyleBrickFilter>,
) -> Result<Vec<style_brick::StyleBrick>> {
    let conn = state.conn();
    let profile_id = active_profile_id(&conn)?;
    style_brick::list(&conn, &profile_id, &filter.unwrap_or_default())
}

/// How many bricks stand under each type, for the counts beside the filter.
#[tauri::command]
pub fn style_brick_counts(state: State<'_, AppState>) -> Result<Vec<(String, i64)>> {
    let conn = state.conn();
    let profile_id = active_profile_id(&conn)?;
    style_brick::counts(&conn, &profile_id)
}

/// One brick with its references — what the editing screen opens.
#[tauri::command]
pub fn get_style_brick(
    state: State<'_, AppState>,
    id: String,
) -> Result<Option<style_brick::StyleBrick>> {
    let conn = state.conn();
    style_brick::get(&conn, &id)
}

/// The pictures a brick was described from.
#[tauri::command]
pub fn style_brick_references(state: State<'_, AppState>, id: String) -> Result<Vec<asset::Asset>> {
    let conn = state.conn();
    asset::for_style_brick(&conn, &id)
}

#[tauri::command]
pub fn create_style_brick(
    state: State<'_, AppState>,
    brick: style_brick::NewStyleBrick,
) -> Result<style_brick::StyleBrick> {
    let mut conn = state.conn();
    let profile_id = active_profile_id(&conn)?;

    let minted = Minted::fresh();
    let logged = operation::Intent::new("style.create")
        .in_profile(&profile_id)
        .param("profile", profile_key(&conn, &profile_id)?)
        .param("brick", serde_json::to_value(&brick)?)
        .minted(&minted);

    recording(&mut conn, logged, |tx| {
        style_brick::create_minted(tx, &profile_id, brick, minted)
    })
}

#[tauri::command]
pub fn update_style_brick(
    state: State<'_, AppState>,
    id: String,
    patch: style_brick::StyleBrickPatch,
) -> Result<style_brick::StyleBrick> {
    let mut conn = state.conn();
    let profile_id = active_profile_id(&conn)?;
    let before = style_brick::get(&conn, &id)?;

    let at = time::now();
    let logged = operation::Intent::new("style.update")
        .in_profile(&profile_id)
        .param("profile", profile_key(&conn, &profile_id)?)
        .param("id", id.clone())
        .param("patch", serde_json::to_value(&patch)?)
        .param("before", was(before.as_ref(), &patch)?)
        .param("at", at.clone());

    recording(&mut conn, logged, |tx| {
        style_brick::update_at(tx, &id, patch, &at)
    })
}

/// A reference picture pasted straight onto a brick.
///
/// The bytes are not written into the log, for the reason a pasted frame's are
/// not: an operation carrying a picture would make the log the size of the
/// pictures. It is recorded as the arrival it is, and not replayed.
#[tauri::command(async)]
pub fn paste_style_reference(
    state: State<'_, AppState>,
    id: String,
    bytes: Vec<u8>,
    name: String,
) -> Result<asset::Asset> {
    let media = state.media_dir()?;
    let mut conn = state.conn();
    let profile_id = active_profile_id(&conn)?;

    let logged = operation::Intent::new("style.attachReference")
        .in_profile(&profile_id)
        .param("profile", profile_key(&conn, &profile_id)?)
        .param("styleBrickId", id.clone())
        .param("source", format!("<pasted: {name}>"));

    recording(&mut conn, logged, |tx| {
        asset::attach_bytes(
            tx,
            &profile_id,
            &media,
            &bytes,
            &name,
            asset::NewAsset {
                style_brick_id: Some(id.clone()),
                ..asset::NewAsset::default()
            },
        )
    })
}

/// Keep an answer as a brick's description, and let it out of draft.
#[tauri::command]
pub fn describe_style_brick(
    state: State<'_, AppState>,
    message_id: String,
    id: String,
) -> Result<style_brick::StyleBrick> {
    let mut conn = state.conn();
    let profile_id = active_profile_id(&conn)?;
    let before = style_brick::get(&conn, &id)?;

    // What stood there, so the write can be taken back. A description read out
    // of a chat message cannot be replayed into a rebuilt workspace, but it
    // can certainly be undone in this one — and an undo that could only blank
    // the text would be worse than none.
    let at = time::now();
    let logged = operation::Intent::new("style.describe")
        .in_profile(&profile_id)
        .param("profile", profile_key(&conn, &profile_id)?)
        .param("messageId", message_id.clone())
        .param("id", id.clone())
        .param(
            "before",
            serde_json::to_value(style_brick::StyleBrickPatch {
                description: Some(before.as_ref().and_then(|one| one.description.clone())),
                status: before.as_ref().map(|one| one.status.clone()),
                ..Default::default()
            })?,
        )
        .param("at", at);

    recording(&mut conn, logged, |tx| {
        assistant::apply::describe_style(tx, &message_id, &id)
    })
}

/// What describing a brick would send, without sending it.
#[tauri::command]
pub fn preview_style_task(
    state: State<'_, AppState>,
    id: String,
    action: String,
) -> Result<assistant::task::Composed> {
    let conn = state.conn();
    assistant::task::compose_for_style(&conn, &id, &action).map(|(composed, _)| composed)
}

/// Describe a brick from its references: the same machinery a work's action
/// uses, aimed at the dictionary instead of a card.
#[tauri::command]
pub fn start_style_task(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
    action: String,
) -> Result<StartedTask> {
    let state = state.inner();
    let runs = Arc::clone(state.runs());
    let workdir = state.assistant_dir();

    // Checked before the chat is opened, for the reason `spawn_task` checks
    // first: a refused duplicate must not leave an empty chat behind for every
    // impatient second click.
    let key = assistant::task::style_key(&action, &id);
    if runs.task_running(&key) {
        return Err(Error::AlreadyRunning);
    }

    let prepared = {
        let conn = state.conn();
        assistant::task::prepare_for_style(&conn, &id, &action)?
    };
    launch(&app, &runs, workdir.as_deref(), prepared)
}

/// Start a prepared task and put a thread on it: the run is recorded, the
/// CLI spawned, and the call returns while it answers. What the style, the
/// comment and the screenshot tasks share; a work's task has its own path
/// because the queue starts it too.
fn launch(
    app: &AppHandle,
    runs: &Arc<assistant_run::Runs>,
    workdir: Option<&std::path::Path>,
    prepared: assistant::task::Prepared,
) -> Result<StartedTask> {
    let (run, stream) = {
        let state = app.state::<AppState>();
        let conn = state.conn();
        assistant_run::start_as(
            &conn,
            runs,
            &prepared.chat_id,
            &prepared.prompt,
            workdir,
            Some(prepared.key.clone()),
            &prepared.attachments,
        )?
    };

    let sink: Arc<dyn Sink> = Arc::new(WindowSink(app.clone()));
    let started = run.clone();
    let handle = app.clone();
    let runs = Arc::clone(runs);

    std::thread::spawn(move || {
        {
            let inner = handle.clone();
            let open = move || inner.state::<AppState>().inner().open_alongside();
            assistant_run::pump(&runs, &sink, &started, &stream, open);
        }
        drain_queue(&handle);
    });

    Ok(StartedTask {
        chat_id: prepared.chat_id,
        run_id: run.id,
        task_key: prepared.key,
        title: prepared.title,
    })
}

/// What drafting a reply to a comment would send, without sending it.
#[tauri::command]
pub fn preview_comment_task(
    state: State<'_, AppState>,
    id: String,
    action: String,
) -> Result<assistant::task::Composed> {
    let conn = state.conn();
    assistant::task::compose_for_comment(&conn, &id, &action).map(|(composed, _)| composed)
}

/// Draft a reply to a comment in the background, in the voice of its channel.
/// The answer arrives as a proposal the comment shows; nothing is written
/// until the person keeps it.
#[tauri::command]
pub fn start_comment_task(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
    action: String,
) -> Result<StartedTask> {
    let state = state.inner();
    let runs = Arc::clone(state.runs());
    let workdir = state.assistant_dir();

    // Checked before the chat is opened, for the reason `spawn_task` checks.
    let key = assistant::task::comment_key(&action, &id);
    if runs.task_running(&key) {
        return Err(Error::AlreadyRunning);
    }

    let prepared = {
        let conn = state.conn();
        assistant::task::prepare_for_comment(&conn, &id, &action)?
    };
    launch(&app, &runs, workdir.as_deref(), prepared)
}

/// How long a pasted screenshot is kept in the temporary folder. Long
/// enough for its run to finish and be retried; after that the comment, if
/// one was kept, is its text, and the picture is not what anyone needs.
const SCREENSHOT_LIFETIME: std::time::Duration = std::time::Duration::from_secs(24 * 60 * 60);

/// Read a pasted screenshot of a comment in the background.
///
/// The picture goes to a temporary folder outside the workspace — it is a
/// way in, not something to keep (migration 0027) — and is read by the
/// profile's action into a proposal the comments screen offers to keep. The
/// channel and the work are where it was pasted.
#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub fn start_screenshot_task(
    app: AppHandle,
    state: State<'_, AppState>,
    bytes: Vec<u8>,
    name: String,
    channel: String,
    work_id: Option<String>,
    action: String,
    today: String,
) -> Result<StartedTask> {
    if bytes.is_empty() {
        return Err(Error::Other("the clipboard held no picture".into()));
    }
    let state = state.inner();
    let runs = Arc::clone(state.runs());
    let workdir = state.assistant_dir();

    let folder = std::env::temp_dir().join("kilna-comment-screenshots");
    std::fs::create_dir_all(&folder).map_err(|cause| {
        Error::Other(format!(
            "could not prepare a place for the screenshot: {cause}"
        ))
    })?;
    sweep_screenshots(&folder);
    let extension = std::path::Path::new(&name)
        .extension()
        .and_then(|ext| ext.to_str())
        .filter(|ext| ext.chars().all(|c| c.is_ascii_alphanumeric()) && ext.len() <= 5)
        .unwrap_or("png")
        .to_ascii_lowercase();
    let path = folder.join(format!("{}.{extension}", uuid::Uuid::new_v4()));
    std::fs::write(&path, &bytes)
        .map_err(|cause| Error::Other(format!("could not keep the screenshot: {cause}")))?;

    let prepared = {
        let conn = state.conn();
        assistant::task::prepare_for_screenshot(
            &conn,
            &action,
            &assistant::task::Screenshot {
                path: &path,
                channel: &channel,
                work_id: work_id.as_deref(),
                today: &today,
            },
        )?
    };
    launch(&app, &runs, workdir.as_deref(), prepared)
}

/// Remove screenshots older than their lifetime. Best effort: a file that
/// cannot be removed now will be tried again at the next paste.
fn sweep_screenshots(folder: &std::path::Path) {
    let Ok(entries) = std::fs::read_dir(folder) else {
        return;
    };
    for entry in entries.flatten() {
        let stale = entry
            .metadata()
            .and_then(|meta| meta.modified())
            .ok()
            .and_then(|at| at.elapsed().ok())
            .is_some_and(|age| age > SCREENSHOT_LIFETIME);
        if stale {
            let _ = std::fs::remove_file(entry.path());
        }
    }
}

/// Every comment read off a screenshot and every drafted reply that waits
/// for the person, with what each says: what the comments screen offers to
/// keep.
#[tauri::command]
pub fn pending_comment_proposals(
    state: State<'_, AppState>,
) -> Result<Vec<assistant::apply::CommentProposal>> {
    let conn = state.conn();
    let profile_id = active_profile_id(&conn)?;
    assistant::apply::pending_comments(&conn, &profile_id)
}

/// Move a brick to the trash, with the pictures it was described from.
///
/// It used to be a plain DELETE under its own `style.delete` operation: no
/// entry to restore, nothing for undo to take back, and the pictures went with
/// it - the one deletion in the app that could not be walked back, sitting in
/// a dialog one button away from Save. The trash is the one road every other
/// deletion takes, so this takes it too, under the one `entity.discard`
/// operation every trash entity shares.
#[tauri::command]
pub fn delete_style_brick(state: State<'_, AppState>, id: String) -> Result<String> {
    discard_and_record(&state, trash::Entity::Style, &id)
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
    let work = work::get(&conn, &work_id)?.ok_or_else(|| Error::not_found("work", &work_id))?;
    Ok(profile.config.verdicts(&work.kind, &latest.axes))
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

/// What a release says about itself, field by field.
///
/// The fields are the release kind's, so a clip is asked for a title, a
/// description, tags and a pinned comment, and a beta read is asked for
/// nothing at all -- see [`release_meta`].
#[tauri::command]
pub fn release_fields(state: State<'_, AppState>, id: String) -> Result<Vec<release_meta::Field>> {
    let conn = state.conn();
    release_meta::fields(&conn, &id)
}

/// Write a release's metadata.
///
/// Goes through the same patch the rest of the tab writes through, which is
/// what gives it undo and the operation log without a line of its own: `meta`
/// is a column of the release, and editing it is editing the release. The
/// values arrive as text because that is what every box on the screen holds,
/// and what every platform they are bound for accepts.
///
/// Keys the profile does not declare are refused rather than stored. The map
/// is open on purpose -- an agent's package and an older profile may both
/// have left things in it, and those are kept -- but a *typed* key that no
/// field names could only come from a bug, and storing it would leave a value
/// no screen can ever show again.
#[tauri::command]
pub fn set_release_fields(
    state: State<'_, AppState>,
    id: String,
    values: std::collections::BTreeMap<String, String>,
) -> Result<Release> {
    let mut conn = state.conn();
    let profile_id = active_profile_id(&conn)?;

    let before = release::get(&conn, &id)?.ok_or_else(|| Error::not_found("release", &id))?;
    let known = release_meta::fields(&conn, &id)?;
    for key in values.keys() {
        if !known.iter().any(|field| &field.key == key) {
            return Err(Error::not_found("release field", key));
        }
    }

    let mut meta = before.meta.clone();
    for (key, value) in &values {
        meta.insert(key.clone(), serde_json::Value::String(value.clone()));
    }

    let patch = ReleasePatch {
        meta: Some(meta),
        ..ReleasePatch::default()
    };

    let at = time::now();
    let logged = operation::Intent::new("release.update")
        .in_profile(&profile_id)
        .param("profile", profile_key(&conn, &profile_id)?)
        .param("id", id.clone())
        .param("patch", serde_json::to_value(&patch)?)
        .param("before", was(Some(&before), &patch)?)
        .param("at", at.clone());

    recording(&mut conn, logged, |tx| {
        release::update_at(tx, &id, patch, &at)
    })
}

/// What the profile would write in a release's fields, without writing it.
///
/// Shown before it lands, because a generated description replaces one
/// someone may have edited by hand, and a button that overwrites without
/// showing what it is about to write is a button people stop pressing.
#[tauri::command]
pub fn preview_release_fields(
    state: State<'_, AppState>,
    id: String,
) -> Result<release_meta::Generated> {
    let conn = state.conn();
    release_meta::generate(&conn, &id)
}

/// Fill a release's fields from the profile's templates.
///
/// Only templated fields are touched; what has no template stays as it was
/// typed. Fields the renderer refused are reported back rather than written
/// blank -- see [`release_meta::generate`].
#[tauri::command]
pub fn generate_release_fields(
    state: State<'_, AppState>,
    id: String,
) -> Result<release_meta::Generated> {
    let mut conn = state.conn();
    let profile_id = active_profile_id(&conn)?;

    let before = release::get(&conn, &id)?.ok_or_else(|| Error::not_found("release", &id))?;
    let generated = release_meta::generate(&conn, &id)?;

    // Nothing rendered: the release is left exactly as it was, and no
    // operation is logged. An undo entry for a change that did not happen is
    // a step the user has to walk back past for nothing.
    if generated.values.is_empty() {
        return Ok(generated);
    }

    let patch = ReleasePatch {
        meta: Some(release_meta::merged(&before.meta, &generated.values)),
        ..ReleasePatch::default()
    };

    let at = time::now();
    let logged = operation::Intent::new("release.update")
        .in_profile(&profile_id)
        .param("profile", profile_key(&conn, &profile_id)?)
        .param("id", id.clone())
        .param("patch", serde_json::to_value(&patch)?)
        .param("before", was(Some(&before), &patch)?)
        .param("at", at.clone());

    recording(&mut conn, logged, |tx| {
        release::update_at(tx, &id, patch, &at)
    })?;

    Ok(generated)
}

/// What a batch generation did, and to what.
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GeneratedBatch {
    /// Releases that gained at least one field.
    pub filled: usize,
    /// Releases whose kind declares no templated field at all: nothing was
    /// asked of them, and nothing happened. Not an error -- this is the
    /// reason the number is smaller than the selection.
    pub skipped: usize,
    /// Fields that could not be rendered, with the release they belong to.
    /// Carried out of the batch rather than counted, because "seven releases
    /// are waiting for lyrics" is only useful if you can see which seven.
    pub refused: Vec<BatchRefusal>,
}

/// A field a batch could not fill, named by the work it is on.
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchRefusal {
    pub release_id: String,
    pub work_title: String,
    pub label: Label,
    pub reason: String,
}

/// Fill the fields of several releases at once.
///
/// The point of the batch is a week of the calendar: a person who has planned
/// six videos writes their metadata in one go or not at all. Each release is
/// generated exactly as it would be alone, and one release failing does not
/// take the rest with it -- the same rule every batch here follows.
///
/// One journal line for the whole batch, not one per release.
#[tauri::command]
pub fn generate_release_fields_batch(
    state: State<'_, AppState>,
    ids: Vec<String>,
) -> Result<GeneratedBatch> {
    let mut conn = state.conn();
    let profile_id = active_profile_id(&conn)?;

    let mut filled = 0usize;
    let mut skipped = 0usize;
    let mut refused: Vec<BatchRefusal> = Vec::new();

    for id in &ids {
        let Some(before) = release::get(&conn, id)? else {
            skipped += 1;
            continue;
        };
        let generated = match release_meta::generate(&conn, id) {
            Ok(generated) => generated,
            Err(cause) => {
                eprintln!("release fields: {id} could not be generated: {cause}");
                skipped += 1;
                continue;
            }
        };

        if !generated.refused.is_empty() {
            let title = journal::work_title(&conn, &before.work_id).unwrap_or_default();
            for refusal in &generated.refused {
                refused.push(BatchRefusal {
                    release_id: id.clone(),
                    work_title: title.clone(),
                    label: refusal.label.clone(),
                    reason: refusal.reason.clone(),
                });
            }
        }

        if generated.values.is_empty() {
            skipped += 1;
            continue;
        }

        let patch = ReleasePatch {
            meta: Some(release_meta::merged(&before.meta, &generated.values)),
            ..ReleasePatch::default()
        };

        let at = time::now();
        let logged = operation::Intent::new("release.update")
            .in_profile(&profile_id)
            .param("profile", profile_key(&conn, &profile_id)?)
            .param("id", id.clone())
            .param("patch", serde_json::to_value(&patch)?)
            .param("before", was(Some(&before), &patch)?)
            .param("at", at.clone());

        match recording(&mut conn, logged, |tx| {
            release::update_at(tx, id, patch, &at)
        }) {
            Ok(_) => filled += 1,
            Err(cause) => {
                eprintln!("release fields: {id} could not be written: {cause}");
                skipped += 1;
            }
        }
    }

    if filled > 0 {
        journal::record(
            &conn,
            &profile_id,
            Record::new("release.fieldsBatch")
                .param("count", i64::try_from(filled).unwrap_or(i64::MAX)),
        );
    }

    Ok(GeneratedBatch {
        filled,
        skipped,
        refused,
    })
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

/// What a work was made from, and what was made from it.
#[tauri::command]
pub fn list_links(state: State<'_, AppState>, work_id: String) -> Result<Links> {
    let conn = state.conn();
    link::for_work(&conn, &work_id)
}

/// Say that a work was made from another, remembering the source's version.
#[tauri::command]
pub fn create_link(state: State<'_, AppState>, link: NewLink) -> Result<link::Link> {
    let mut conn = state.conn();
    let profile_id = active_profile_id(&conn)?;
    record_link(&mut conn, &profile_id, link)
}

/// The body of `create_link`, shared with `derive_work`: the operation, the
/// row and the journal line, in that order.
fn record_link(
    conn: &mut rusqlite::Connection,
    profile_id: &str,
    new: NewLink,
) -> Result<link::Link> {
    let minted = Minted::fresh();
    let logged = operation::Intent::new("link.create")
        .in_profile(profile_id)
        .param("profile", profile_key(conn, profile_id)?)
        .param("link", serde_json::to_value(&new)?)
        .minted(&minted);

    let created = recording(conn, logged, |tx| {
        link::create_minted(tx, profile_id, new, minted)
    })?;

    journal::record(
        conn,
        profile_id,
        Record::new("link.created")
            .param(
                "title",
                journal::work_title(conn, &created.work_id).unwrap_or_default(),
            )
            .param("source", created.source_title.clone())
            .about("work", created.work_id.clone()),
    );

    Ok(created)
}

/// Unsay it. The link goes outright, tombstoned by the schema: it is a fact
/// about two works, not a thing with a body worth a drawer in the trash.
#[tauri::command]
pub fn delete_link(state: State<'_, AppState>, id: String) -> Result<()> {
    let mut conn = state.conn();
    let profile_id = active_profile_id(&conn)?;
    let before = link::get(&conn, &id)?.ok_or_else(|| Error::not_found("link", &id))?;

    // The row itself goes into the log, so an undo can put it back under the
    // same id and moment: a link has no drawer in the trash to come back from.
    let logged = operation::Intent::new("link.delete")
        .in_profile(&profile_id)
        .param("profile", profile_key(&conn, &profile_id)?)
        .param("id", id.clone())
        .param(
            "before",
            serde_json::json!({
                "work_id": before.work_id,
                "source_id": before.source_id,
                "role": before.role,
                "source_version_id": before.source_version_id,
                "created_at": before.created_at,
            }),
        );

    recording(&mut conn, logged, |tx| link::delete(tx, &id))?;

    journal::record(
        &conn,
        &profile_id,
        Record::new("link.removed")
            .param(
                "title",
                journal::work_title(&conn, &before.work_id).unwrap_or_default(),
            )
            .param("source", before.source_title)
            .about("work", before.work_id),
    );

    Ok(())
}

/// Make a work from another: a video from a song.
///
/// The new work takes the source's title and the overview fields the profile
/// has — the inputs flow once, at creation, and never again (decision of
/// 2026-09-10): what the source does afterwards is a fact the link reports,
/// not a change pushed into the work. Its text is not copied: a video's roles
/// are its own. Two operations, as a hand would make them — the work, then
/// the link — so a replay rebuilds both under their ids.
#[tauri::command]
pub fn derive_work(
    state: State<'_, AppState>,
    source_id: String,
    kind: String,
    title: Option<String>,
) -> Result<Work> {
    let mut conn = state.conn();
    let profile_id = active_profile_id(&conn)?;
    let source =
        work::get(&conn, &source_id)?.ok_or_else(|| Error::not_found("work", &source_id))?;
    let config = profile::config_for(&conn, &profile_id)?;
    if config.kind(&kind).is_none() {
        return Err(Error::Other(format!(
            "the profile has no kind of work `{kind}`"
        )));
    }
    // Only fields the profile has: a stray key in the source's meta is not
    // carried into a new work.
    let meta: serde_json::Map<String, serde_json::Value> = source
        .meta
        .iter()
        .filter(|(key, _)| {
            config
                .work_meta_fields
                .iter()
                .any(|field| field.key == **key)
        })
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect();
    let new = NewWork {
        kind,
        title: title
            .map(|title| title.trim().to_owned())
            .filter(|title| !title.is_empty())
            .unwrap_or_else(|| source.title.clone()),
        meta: Some(meta),
        ..NewWork::default()
    };

    let minted = Minted::fresh();
    let logged = operation::Intent::new("work.create")
        .in_profile(&profile_id)
        .param("profile", profile_key(&conn, &profile_id)?)
        .param("work", serde_json::to_value(&new)?)
        .minted(&minted);
    let created = recording(&mut conn, logged, |tx| {
        work::create_minted(tx, &profile_id, new, minted)
    })?;
    journal::record(
        &conn,
        &profile_id,
        Record::new("work.created")
            .param("title", created.title.clone())
            .about("work", created.id.clone()),
    );

    record_link(
        &mut conn,
        &profile_id,
        NewLink {
            work_id: created.id.clone(),
            source_id,
            role: None,
            source_version_id: None,
        },
    )?;

    Ok(created)
}

/// The storyboard of a work, in order.
#[tauri::command]
pub fn list_scenes(state: State<'_, AppState>, work_id: String) -> Result<Vec<Scene>> {
    let conn = state.conn();
    scene::for_work(&conn, &work_id)
}

/// Add a scene to a work's storyboard — after the last, unless numbered.
#[tauri::command]
pub fn create_scene(state: State<'_, AppState>, scene: NewScene) -> Result<Scene> {
    let mut conn = state.conn();
    let profile_id = active_profile_id(&conn)?;

    let minted = Minted::fresh();
    let logged = operation::Intent::new("scene.create")
        .in_profile(&profile_id)
        .param("profile", profile_key(&conn, &profile_id)?)
        .param("scene", serde_json::to_value(&scene)?)
        .minted(&minted);

    let created = recording(&mut conn, logged, |tx| {
        scene::create_minted(tx, &profile_id, scene, minted)
    })?;

    journal::record(
        &conn,
        &profile_id,
        Record::new("scene.created")
            .param(
                "title",
                journal::work_title(&conn, &created.work_id).unwrap_or_default(),
            )
            .param("number", created.position)
            .about("work", created.work_id.clone()),
    );

    Ok(created)
}

/// Edit a scene: one field, or the prompt blocks as a set.
#[tauri::command]
pub fn update_scene(state: State<'_, AppState>, id: String, patch: ScenePatch) -> Result<Scene> {
    let mut conn = state.conn();
    let profile_id = active_profile_id(&conn)?;
    let before = scene::get(&conn, &id)?;

    let at = time::now();
    let logged = operation::Intent::new("scene.update")
        .in_profile(&profile_id)
        .param("profile", profile_key(&conn, &profile_id)?)
        .param("id", id.clone())
        .param("patch", serde_json::to_value(&patch)?)
        .param("before", was(before.as_ref(), &patch)?)
        .param("at", at.clone());

    recording(&mut conn, logged, |tx| {
        scene::update_at(tx, &id, patch, &at)
    })
}

/// What every scene of a board is about: the people in it, the places.
#[tauri::command]
pub fn list_scene_notes(state: State<'_, AppState>, work_id: String) -> Result<Vec<SceneNote>> {
    let conn = state.conn();
    scene_note::for_work(&conn, &work_id)
}

/// Say that a scene is about a note — a character, a place.
#[tauri::command]
pub fn attach_scene_note(
    state: State<'_, AppState>,
    scene_id: String,
    note_id: String,
) -> Result<SceneNote> {
    let mut conn = state.conn();
    let profile_id = active_profile_id(&conn)?;

    let minted = Minted::fresh();
    let logged = operation::Intent::new("scene.attachNote")
        .in_profile(&profile_id)
        .param("profile", profile_key(&conn, &profile_id)?)
        .param("sceneId", scene_id.clone())
        .param("noteId", note_id.clone())
        .minted(&minted);

    recording(&mut conn, logged, |tx| {
        scene_note::attach_minted(tx, &scene_id, &note_id, minted)
    })
}

/// Stop a scene being about a note.
#[tauri::command]
pub fn detach_scene_note(
    state: State<'_, AppState>,
    scene_id: String,
    note_id: String,
) -> Result<()> {
    let mut conn = state.conn();
    let profile_id = active_profile_id(&conn)?;

    let logged = operation::Intent::new("scene.detachNote")
        .in_profile(&profile_id)
        .param("profile", profile_key(&conn, &profile_id)?)
        .param("sceneId", scene_id.clone())
        .param("noteId", note_id.clone());

    recording(&mut conn, logged, |tx| {
        scene_note::detach(tx, &scene_id, &note_id)
    })
}

/// Build the board's frame from the parts the source text marks out.
///
/// One scene per part, in the text's order, carrying the part's name. The
/// scenes' ids are minted here and travel in the operation, so a workspace
/// rebuilt from the log lands on the same rows.
#[tauri::command]
pub fn frame_scenes(
    state: State<'_, AppState>,
    work_id: String,
    role: String,
) -> Result<Vec<Scene>> {
    let mut conn = state.conn();
    let profile_id = active_profile_id(&conn)?;

    // How many parts the text marks out right now: the ids are minted for
    // exactly those, and framing refuses if the text has changed since.
    let parts = scene::parts_of_source(&conn, &work_id, &role)?;
    let at = time::now();
    let minted: Vec<Minted> = (0..parts).map(|_| Minted::fresh()).collect();
    let ids: Vec<String> = minted.iter().map(|one| one.id().to_owned()).collect();

    let logged = operation::Intent::new("scene.frame")
        .in_profile(&profile_id)
        .param("profile", profile_key(&conn, &profile_id)?)
        .param("workId", work_id.clone())
        .param("role", role.clone())
        .param("ids", serde_json::to_value(&ids)?)
        .param("at", at.clone());

    let framed = scene::frame_from_text(&mut conn, &work_id, &role, &minted, Some(logged))?;

    journal::record(
        &conn,
        &profile_id,
        Record::new("scene.framed")
            .param(
                "title",
                journal::work_title(&conn, &work_id).unwrap_or_default(),
            )
            .param("count", framed.len() as i64)
            .about("work", work_id.clone()),
    );

    Ok(framed)
}

/// Divide the work's length between the scenes of its board.
///
/// The first timing of a board, not the last word on it: every span after
/// this is dragged by hand. One operation for the whole board — see
/// `scene::time_board_at`.
#[tauri::command]
pub fn time_scenes(state: State<'_, AppState>, work_id: String) -> Result<Vec<Scene>> {
    let mut conn = state.conn();
    let profile_id = active_profile_id(&conn)?;

    // The spans as they stand, so the undo puts back exactly these.
    let before: Vec<serde_json::Value> = scene::for_work(&conn, &work_id)?
        .into_iter()
        .map(|scene| {
            serde_json::json!({
                "id": scene.id,
                "startsAt": scene.starts_at,
                "endsAt": scene.ends_at,
            })
        })
        .collect();

    let at = time::now();
    let logged = operation::Intent::new("scene.time")
        .in_profile(&profile_id)
        .param("profile", profile_key(&conn, &profile_id)?)
        .param("workId", work_id.clone())
        .param("before", serde_json::to_value(&before)?)
        .param("at", at.clone());

    let timed = scene::time_board_at(&mut conn, &work_id, &at, Some(logged))?;

    journal::record(
        &conn,
        &profile_id,
        Record::new("scene.timed")
            .param(
                "title",
                journal::work_title(&conn, &work_id).unwrap_or_default(),
            )
            .param("count", timed.len() as i64)
            .about("work", work_id.clone()),
    );

    Ok(timed)
}

/// Number a board in the order given: 1..N, in one change.
///
/// The whole order travels rather than one scene and a target number, because
/// both gestures the screen offers — putting a new scene between two others,
/// and moving one that is already there — are the same thing said twice, and
/// the list says it once. The order the board held travels in the operation,
/// so the undo is this same call with that list (see `scene::renumber`).
#[tauri::command]
pub fn renumber_scenes(
    state: State<'_, AppState>,
    work_id: String,
    ids: Vec<String>,
) -> Result<Vec<Scene>> {
    let mut conn = state.conn();
    let profile_id = active_profile_id(&conn)?;

    // The numbers as they stand, so the undo puts back exactly these — not
    // a tidy 1..N, which is very likely a board this one has never been.
    let before: Vec<serde_json::Value> = scene::for_work(&conn, &work_id)?
        .into_iter()
        .map(|scene| serde_json::json!({ "id": scene.id, "position": scene.position }))
        .collect();

    let at = time::now();
    let logged = operation::Intent::new("scene.renumber")
        .in_profile(&profile_id)
        .param("profile", profile_key(&conn, &profile_id)?)
        .param("workId", work_id.clone())
        .param("ids", serde_json::to_value(&ids)?)
        .param("before", serde_json::to_value(&before)?)
        .param("at", at.clone());

    let numbered = scene::renumber(&mut conn, &work_id, &ids, &at, Some(logged))?;

    journal::record(
        &conn,
        &profile_id,
        Record::new("scene.renumbered")
            .param(
                "title",
                journal::work_title(&conn, &work_id).unwrap_or_default(),
            )
            .param("count", numbered.len() as i64)
            .about("work", work_id.clone()),
    );

    Ok(numbered)
}

#[tauri::command]
pub fn delete_scene(state: State<'_, AppState>, id: String) -> Result<String> {
    discard_and_record(&state, trash::Entity::Scene, &id)
}

/// The stretches a short is spliced from, in order.
#[tauri::command]
pub fn list_cuts(state: State<'_, AppState>, work_id: String) -> Result<Vec<cut::Cut>> {
    let conn = state.conn();
    cut::for_work(&conn, &work_id)
}

/// What has been cut out of this work — the question a donor's card asks.
#[tauri::command]
pub fn list_cuts_from(state: State<'_, AppState>, source_id: String) -> Result<Vec<cut::Cut>> {
    let conn = state.conn();
    cut::from_source(&conn, &source_id)
}

/// Take a stretch of a source into a short.
#[tauri::command]
pub fn create_cut(state: State<'_, AppState>, cut: cut::NewCut) -> Result<cut::Cut> {
    let mut conn = state.conn();
    let profile_id = active_profile_id(&conn)?;

    let minted = Minted::fresh();
    let logged = operation::Intent::new("cut.create")
        .in_profile(&profile_id)
        .param("profile", profile_key(&conn, &profile_id)?)
        .param("cut", serde_json::to_value(&cut)?)
        .minted(&minted);

    let created = recording(&mut conn, logged, |tx| {
        cut::create_minted(tx, &profile_id, cut, minted)
    })?;

    journal::record(
        &conn,
        &profile_id,
        Record::new("cut.created")
            .param(
                "title",
                journal::work_title(&conn, &created.work_id).unwrap_or_default(),
            )
            .param("source", created.source_title.clone())
            .about("work", created.work_id.clone()),
    );

    Ok(created)
}

/// Move an end of a stretch, renumber it, or name it.
#[tauri::command]
pub fn update_cut(
    state: State<'_, AppState>,
    id: String,
    patch: cut::CutPatch,
) -> Result<cut::Cut> {
    let mut conn = state.conn();
    let profile_id = active_profile_id(&conn)?;
    let before = cut::get(&conn, &id)?;

    let logged = operation::Intent::new("cut.update")
        .in_profile(&profile_id)
        .param("profile", profile_key(&conn, &profile_id)?)
        .param("id", id.clone())
        .param("patch", serde_json::to_value(&patch)?)
        .param("before", was(before.as_ref(), &patch)?);

    recording(&mut conn, logged, |tx| cut::update(tx, &id, patch))
}

/// Put a splice in the order given, 1..N, in one change.
///
/// The whole order travels rather than one stretch and a target number, for
/// the reason `renumber_scenes` gives: both gestures the screen offers are
/// the same thing said twice, and the list says it once. The order the splice
/// held travels in the operation, so the undo is this same call with it.
#[tauri::command]
pub fn reorder_cuts(
    state: State<'_, AppState>,
    work_id: String,
    ids: Vec<String>,
) -> Result<Vec<cut::Cut>> {
    let mut conn = state.conn();
    let profile_id = active_profile_id(&conn)?;

    // The order as it stands, so the undo puts back exactly this.
    let before: Vec<String> = cut::for_work(&conn, &work_id)?
        .into_iter()
        .map(|cut| cut.id)
        .collect();

    let logged = operation::Intent::new("cut.reorder")
        .in_profile(&profile_id)
        .param("profile", profile_key(&conn, &profile_id)?)
        .param("workId", work_id.clone())
        .param("ids", serde_json::to_value(&ids)?)
        .param("before", serde_json::to_value(&before)?);

    recording(&mut conn, logged, |tx| cut::reorder(tx, &work_id, &ids))
}

#[tauri::command]
pub fn delete_cut(state: State<'_, AppState>, id: String) -> Result<String> {
    discard_and_record(&state, trash::Entity::Cut, &id)
}

/// What a short is, told to something that can cut video.
///
/// The core's whole part in making the file: the stretches in order, each
/// beside the donor's video on disk. The plugin of v1.10 reads this and runs
/// ffmpeg — the core does not (decision of 2026-09-11).
#[tauri::command]
pub fn cut_shot_list(state: State<'_, AppState>, work_id: String) -> Result<Vec<cut::Shot>> {
    let conn = state.conn();
    cut::shot_list(&conn, &work_id)
}

/// Copy a file into the workspace and attach it to a work or a release.
///
/// The path comes from the file picker, so it is a place on this machine;
/// the bytes are copied into the workspace's own `media/` directory and the
/// row holds where they landed.
#[tauri::command(async)]
pub fn attach_asset(
    state: State<'_, AppState>,
    source: String,
    asset: asset::NewAsset,
) -> Result<asset::Asset> {
    let media = state.media_dir()?;
    let mut conn = state.conn();
    let profile_id = active_profile_id(&conn)?;

    let minted = Minted::fresh();
    let logged = operation::Intent::new("asset.attach")
        .in_profile(&profile_id)
        .param("profile", profile_key(&conn, &profile_id)?)
        .param("source", source.clone())
        .param("asset", serde_json::to_value(&asset)?)
        .minted(&minted);

    let source = PathBuf::from(&source);
    let attached = recording(&mut conn, logged, |tx| {
        asset::attach_minted(tx, &profile_id, &media, &source, asset, minted)
    })?;

    let named = Record::new("asset.attached").param(
        "name",
        attached
            .original_name
            .clone()
            .or_else(|| attached.label.clone())
            .unwrap_or_default(),
    );
    // A file attached to a release belongs to no work, and the feed's line
    // is about the file either way.
    let entry = match attached.work_id.as_deref() {
        Some(work_id) => named.about("work", work_id.to_owned()),
        None => named,
    };
    journal::record(&conn, &profile_id, entry);

    Ok(attached)
}

/// The files attached to a work, oldest first.
#[tauri::command]
pub fn list_work_assets(state: State<'_, AppState>, work_id: String) -> Result<Vec<asset::Asset>> {
    let conn = state.conn();
    asset::for_work(&conn, &work_id)
}

/// The files attached to a release, oldest first.
#[tauri::command]
pub fn list_release_assets(
    state: State<'_, AppState>,
    release_id: String,
) -> Result<Vec<asset::Asset>> {
    let conn = state.conn();
    asset::for_release(&conn, &release_id)
}

/// The cover of every work that has one, by work id — one read for a
/// catalogue of two hundred rows.
#[tauri::command]
pub fn list_covers(state: State<'_, AppState>) -> Result<Vec<(String, String)>> {
    let conn = state.conn();
    let profile_id = active_profile_id(&conn)?;
    asset::covers_of(&conn, &profile_id)
}

/// Forget a file and remove the copy the workspace made.
///
/// Not the trash: what the trash promises is that a deletion can be taken
/// back, and a row restored beside bytes that are gone is a promise broken.
/// A file is detached and the copy goes with it; the original, wherever it
/// came from, was never touched.
#[tauri::command]
pub fn detach_asset(state: State<'_, AppState>, id: String) -> Result<()> {
    let mut conn = state.conn();
    let profile_id = active_profile_id(&conn)?;

    let logged = operation::Intent::new("asset.detach")
        .in_profile(&profile_id)
        .param("profile", profile_key(&conn, &profile_id)?)
        .param("id", id.clone());

    recording(&mut conn, logged, |tx| asset::delete(tx, &id))
}

/// The frames of every scene of a board, in order — one read for a
/// storyboard of fifty scenes.
#[tauri::command]
pub fn list_scene_frames(
    state: State<'_, AppState>,
    work_id: String,
) -> Result<Vec<scene_frame::SceneFrame>> {
    let conn = state.conn();
    scene_frame::for_work(&conn, &work_id)
}

/// Copy a picture into the workspace and hang it on a scene.
///
/// The path comes from the picker, from a drop, or from a pasted image the
/// window wrote to a temporary file; by the time it gets here it is a place
/// on this machine, and the bytes are copied into the workspace's `media/`.
#[tauri::command(async)]
pub fn attach_scene_frame(
    state: State<'_, AppState>,
    scene_id: String,
    kind: String,
    source: String,
) -> Result<scene_frame::SceneFrame> {
    let media = state.media_dir()?;
    let mut conn = state.conn();
    let profile_id = active_profile_id(&conn)?;

    let minted = Minted::fresh();
    let logged = operation::Intent::new("scene.attachFrame")
        .in_profile(&profile_id)
        .param("profile", profile_key(&conn, &profile_id)?)
        .param("sceneId", scene_id.clone())
        .param("kind", kind.clone())
        .param("source", source.clone())
        .minted(&minted);

    let source = PathBuf::from(&source);
    let attached = recording(&mut conn, logged, |tx| {
        scene_frame::attach_minted(tx, &media, &scene_id, &kind, &source, minted)
    })?;

    let entry = Record::new("scene.framed")
        .param("name", attached.original_name.clone().unwrap_or_default())
        .about("scene", scene_id);
    journal::record(&conn, &profile_id, entry);

    Ok(attached)
}

/// Hang a pasted picture on a scene: the clipboard gives bytes, not a path.
///
/// The window sends the bytes rather than writing a file itself, so the
/// application needs no filesystem permissions for a picture on its way into
/// a directory this process already owns.
#[tauri::command(async)]
pub fn paste_scene_frame(
    state: State<'_, AppState>,
    scene_id: String,
    kind: String,
    bytes: Vec<u8>,
    name: String,
) -> Result<scene_frame::SceneFrame> {
    let media = state.media_dir()?;
    let mut conn = state.conn();
    let profile_id = active_profile_id(&conn)?;

    // The bytes are not written into the log: an operation carrying a picture
    // would make the log the size of the pictures. It is recorded as the
    // arrival it is, and like `scene.attachFrame` it is not replayed.
    let logged = operation::Intent::new("scene.attachFrame")
        .in_profile(&profile_id)
        .param("profile", profile_key(&conn, &profile_id)?)
        .param("sceneId", scene_id.clone())
        .param("kind", kind.clone())
        .param("source", format!("<pasted: {name}>"));

    let attached = recording(&mut conn, logged, |tx| {
        scene_frame::attach_bytes(tx, &media, &scene_id, &kind, &bytes, &name)
    })?;

    let entry = Record::new("scene.framed")
        .param("name", attached.original_name.clone().unwrap_or_default())
        .about("scene", scene_id);
    journal::record(&conn, &profile_id, entry);

    Ok(attached)
}

/// Take a frame off a scene, and the copy the workspace made with it.
///
/// Not the trash, for the reason ADR 0027 gives: a row restored beside bytes
/// that are gone is a broken picture, not an undo.
#[tauri::command]
pub fn detach_scene_frame(state: State<'_, AppState>, id: String) -> Result<()> {
    let mut conn = state.conn();
    let profile_id = active_profile_id(&conn)?;

    let logged = operation::Intent::new("scene.detachFrame")
        .in_profile(&profile_id)
        .param("profile", profile_key(&conn, &profile_id)?)
        .param("id", id.clone());

    recording(&mut conn, logged, |tx| scene_frame::detach(tx, &id))
}

/// Cut the video from this frame — and from no other of its scene.
#[tauri::command]
pub fn select_scene_frame(
    state: State<'_, AppState>,
    id: String,
) -> Result<scene_frame::SceneFrame> {
    let mut conn = state.conn();
    let profile_id = active_profile_id(&conn)?;

    // Which frame the scene was cut from before, so an undo puts that one
    // back rather than leaving the scene undecided — the verdict it had is
    // not the same as no verdict.
    let frame = scene_frame::get(&conn, &id)?
        .ok_or_else(|| crate::error::Error::not_found("scene_frame", &id))?;
    let before = scene_frame::for_scene(&conn, &frame.scene_id)?
        .into_iter()
        .find(|one| one.is_selected)
        .map(|one| one.id);

    let logged = operation::Intent::new("scene.selectFrame")
        .in_profile(&profile_id)
        .param("profile", profile_key(&conn, &profile_id)?)
        .param("id", id.clone())
        .param("sceneId", frame.scene_id.clone())
        .param("before", serde_json::to_value(&before)?);

    recording(&mut conn, logged, |tx| scene_frame::select(tx, &id))
}

/// Go back to having no frame chosen for a scene.
#[tauri::command]
pub fn clear_scene_frame(state: State<'_, AppState>, scene_id: String, kind: String) -> Result<()> {
    let mut conn = state.conn();
    let profile_id = active_profile_id(&conn)?;

    // The verdict being cleared, so an undo can put it back. Of this kind
    // only: the chosen still is not touched by a change of mind about a clip.
    let before = scene_frame::selected(&conn, &scene_id, &kind)?.map(|one| one.id);

    let logged = operation::Intent::new("scene.clearFrame")
        .in_profile(&profile_id)
        .param("profile", profile_key(&conn, &profile_id)?)
        .param("sceneId", scene_id.clone())
        .param("kind", kind.clone())
        .param("before", serde_json::to_value(&before)?);

    recording(&mut conn, logged, |tx| {
        scene_frame::clear_selection(tx, &scene_id, &kind)
    })
}

/// Put a scene's frames in the order given, first to last.
#[tauri::command]
pub fn reorder_scene_frames(
    state: State<'_, AppState>,
    scene_id: String,
    kind: String,
    ids: Vec<String>,
) -> Result<Vec<scene_frame::SceneFrame>> {
    let mut conn = state.conn();
    let profile_id = active_profile_id(&conn)?;

    // The order they stood in, so an undo restores it exactly.
    let before: Vec<String> = scene_frame::of_kind(&conn, &scene_id, &kind)?
        .into_iter()
        .map(|one| one.id)
        .collect();

    let logged = operation::Intent::new("scene.reorderFrames")
        .in_profile(&profile_id)
        .param("profile", profile_key(&conn, &profile_id)?)
        .param("sceneId", scene_id.clone())
        .param("kind", kind.clone())
        .param("ids", serde_json::to_value(&ids)?)
        .param("before", serde_json::to_value(&before)?);

    recording(&mut conn, logged, |tx| {
        scene_frame::reorder(tx, &scene_id, &kind, &ids)
    })
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

/// The works whose text answers a query, best match first.
///
/// What the catalogue's box asks, as opposed to the palette's: it wants the
/// list narrowed to the works that say something, not the half dozen lines
/// that say it. Ids only — the rows are already on the screen.
#[tauri::command]
pub fn works_matching(state: State<'_, AppState>, query: String) -> Result<Vec<String>> {
    let conn = state.conn();
    let profile_id = active_profile_id(&conn)?;
    search::works_matching(&conn, &profile_id, &query)
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

/// A second attempt at a video: the same donor, the same board, its own life.
///
/// Not "a version of the whole video" — a scene belongs to a work, and the
/// second attempt is a second work (decision of 2026-09-11). The first one is
/// left exactly as it was, which is the point: the two are compared.
#[tauri::command]
pub fn clone_work(
    state: State<'_, AppState>,
    work_id: String,
    title: String,
) -> Result<crate::clone::Cloned> {
    let mut conn = state.conn();
    let profile_id = active_profile_id(&conn)?;

    // The new work's id is decided here rather than inside, so the operation
    // carries it: a replay lands the clone under the id everything else
    // already names, and an undo knows which work to discard.
    let minted = Minted::fresh();
    let logged = operation::Intent::new("work.clone")
        .in_profile(&profile_id)
        .param("profile", profile_key(&conn, &profile_id)?)
        .param("workId", work_id.clone())
        .param("title", title.clone())
        .minted(&minted);

    let made = recording(&mut conn, logged, |tx| {
        crate::clone::clone_work_minted(tx, &work_id, &title, minted)
    })?;

    let entry = Record::new("work.cloned")
        .param("title", made.work.title.clone())
        .param("scenes", made.scenes.to_string())
        .about("work", made.work.id.clone());
    journal::record(&conn, &profile_id, entry);

    Ok(made)
}

/// Write a text the window composed to a place the person picked.
///
/// The window has no filesystem permissions and is not given any for this:
/// it already holds the text — the montage list is rendered from what the
/// board is showing — and what it lacks is the right to write a file. So the
/// text comes here and the backend writes it, the same division the pasted
/// frame settled in v0.68.
///
/// The path is the one the save dialog returned, so the person chose it; this
/// refuses only to write a directory, which the dialog cannot return but a
/// caller could pass.
#[tauri::command(async)]
pub fn write_text_file(path: String, text: String) -> Result<String> {
    let target = std::path::Path::new(&path);
    if target.is_dir() {
        return Err(crate::error::Error::Other(format!(
            "{path} is a directory, not a file"
        )));
    }
    std::fs::write(target, text)
        .map_err(|cause| crate::error::Error::Other(format!("could not write {path}: {cause}")))?;
    Ok(path)
}

/// Write the active profile out as markdown.
#[tauri::command(async)]
pub fn export_markdown(state: State<'_, AppState>, directory: String) -> Result<ExportReport> {
    let conn = state.conn();
    export::to_markdown(&conn, std::path::Path::new(&directory))
}

/// Pack a work into a folder: its board with every prompt, the pictures under
/// names that say what they are, and what its releases go out as.
///
/// Reads only — nothing about the work changes, so there is no operation to
/// record. What it writes is outside the workspace entirely.
#[tauri::command(async)]
pub fn export_package(
    state: State<'_, AppState>,
    work_id: String,
    directory: String,
) -> Result<package::PackageReport> {
    let conn = state.conn();
    package::write(&conn, &work_id, std::path::Path::new(&directory))
}

/// Whether a work has anything worth packing: a board, or something written
/// about a release. What the button hangs on, so a song with neither is not
/// offered a folder holding one nearly empty page.
#[tauri::command]
pub fn can_export_package(state: State<'_, AppState>, work_id: String) -> Result<bool> {
    let conn = state.conn();
    package::has_anything(&conn, &work_id)
}

/// Copy the workspace somewhere safe — the database and the files with it.
#[tauri::command(async)]
///
/// Off the main thread, like every command below whose work is files: a
/// synchronous command runs on the thread that draws the window, and a backup
/// copying a folder of clips froze it for as long as the copy took. The
/// database is held only while SQLite copies it; the pictures are copied with
/// the lock let go, so the rest of the app keeps working meanwhile.
pub fn backup_workspace(state: State<'_, AppState>, destination: String) -> Result<String> {
    // Asked for before the connection is taken, because preparing it may
    // create the directory and that is not work to do under the lock.
    let media = state.media_dir().ok();
    let destination = std::path::Path::new(&destination);
    let written = {
        let conn = state.conn();
        backup::write_database(&conn, destination)?
    };
    backup::copy_media(destination, media.as_deref())?;
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
#[tauri::command(async)]
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
#[tauri::command(async)]
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
        let mut conn = state.conn();
        let profile_id = active_profile_id(&conn)?;
        let at = time::now();

        // Merged into the fields as they are NOW, read after the plugin
        // returned rather than taken from what it was sent: a field edited
        // while it ran would otherwise be put back to what it was when it
        // started. And written as the ordinary edit it is - through the log,
        // with what it replaced - so undo takes back what the plugin wrote.
        // It used to write past the log, and Ctrl+Z after a plugin took back
        // the person's previous edit instead.
        match target {
            Target::Release => {
                let before = release::get(&conn, &id)?
                    .ok_or_else(|| Error::not_found("release", id.clone()))?;
                let patch = ReleasePatch {
                    meta: Some(plugin::merge_meta(&before.meta, &outcome.meta)),
                    ..ReleasePatch::default()
                };
                let logged = operation::Intent::new("release.update")
                    .in_profile(&profile_id)
                    .param("profile", profile_key(&conn, &profile_id)?)
                    .param("id", id.clone())
                    .param("patch", serde_json::to_value(&patch)?)
                    .param("before", was(Some(&before), &patch)?)
                    .param("at", at.clone());
                recording(&mut conn, logged, |tx| {
                    release::update_at(tx, &id, patch, &at)
                })?;
            }
            Target::Work => {
                let before =
                    work::get(&conn, &id)?.ok_or_else(|| Error::not_found("work", id.clone()))?;
                let patch = WorkPatch {
                    meta: Some(plugin::merge_meta(&before.meta, &outcome.meta)),
                    ..WorkPatch::default()
                };
                let logged = operation::Intent::new("work.update")
                    .in_profile(&profile_id)
                    .param("profile", profile_key(&conn, &profile_id)?)
                    .param("id", id.clone())
                    .param("patch", serde_json::to_value(&patch)?)
                    .param("before", was(Some(&before), &patch)?)
                    .param("at", at.clone());
                recording(&mut conn, logged, |tx| work::update_at(tx, &id, patch, &at))?;
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
/// The command that registers this build with Claude Code, for the settings
/// screen to show. Read-only: it names this executable, nothing more.
#[tauri::command]
pub fn mcp_registration() -> Result<String> {
    crate::mcp::registration_command()
}

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

/// Apply what a message proposes — a version, a score, a note, a whole
/// package — and mark the message applied.
///
/// One command for every kind, so the chat's buttons and *apply all* go the
/// same way and the mark is the same mark. Records its operations one level
/// down, one per row written, inside `assistant::apply` — the same intents
/// the hand-driven commands above record, so a replay cannot tell them apart.
#[tauri::command]
pub fn apply_proposal(
    state: State<'_, AppState>,
    message_id: String,
    overrides: Option<assistant::apply::Overrides>,
) -> Result<assistant::apply::Outcome> {
    let mut conn = state.conn();
    let profile_id = active_profile_id(&conn)?;
    assistant::apply::apply(
        &mut conn,
        &profile_id,
        &message_id,
        overrides.unwrap_or_default(),
    )
}

/// Apply every proposal in a chat nobody has applied yet — one click for a
/// week of an agent's suggestions. Records its operations as `apply_proposal`
/// does, one per row.
#[tauri::command]
pub fn apply_pending_proposals(
    state: State<'_, AppState>,
    chat_id: String,
) -> Result<Vec<assistant::apply::Outcome>> {
    let mut conn = state.conn();
    let profile_id = active_profile_id(&conn)?;
    assistant::apply::apply_pending(&mut conn, &profile_id, &chat_id)
}

/// Every proposal waiting for an answer, across every chat of the profile.
///
/// What the bell reads. A proposal is answered by applying it or by turning
/// it down, and until then it is work the assistant has done that nobody has
/// looked at — which is exactly what a notification is for.
#[tauri::command]
pub fn pending_proposals(state: State<'_, AppState>) -> Result<Vec<assistant::apply::Pending>> {
    let conn = state.conn();
    let profile_id = active_profile_id(&conn)?;
    assistant::apply::pending(&conn, &profile_id)
}

/// Turn a proposal down. The answer stays in the chat; it stops waiting.
#[tauri::command]
pub fn dismiss_proposal(state: State<'_, AppState>, message_id: String) -> Result<()> {
    let conn = state.conn();
    assistant::apply::dismiss(&conn, &message_id)
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

/// What a task is about, as the window sends it.
///
/// One argument rather than four: the version, the scene, the block and the
/// reference files travel together everywhere else, and a command with eight
/// parameters is one where a caller swaps two of the same type without the
/// compiler noticing.
#[derive(Default, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskAbout {
    #[serde(default)]
    pub version_id: Option<String>,
    #[serde(default)]
    pub scene_id: Option<String>,
    /// One prompt block of that scene, when the action is aimed at one.
    #[serde(default)]
    pub block: Option<String>,
    #[serde(default)]
    pub attachments: Option<Vec<String>>,
    /// The style bricks picked for this run, in the order they were picked.
    #[serde(default)]
    pub style_brick_ids: Option<Vec<String>>,
}

impl TaskAbout {
    /// The borrowed form the task module reads.
    fn as_about<'a>(
        &'a self,
        attachments: &'a [String],
        style_brick_ids: &'a [String],
    ) -> assistant::task::About<'a> {
        assistant::task::About {
            version_id: self.version_id.as_deref(),
            scene_id: self.scene_id.as_deref(),
            block: self.block.as_deref(),
            attachments,
            style_brick_ids,
        }
    }
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
    about: Option<TaskAbout>,
) -> Result<StartedTask> {
    let about = about.unwrap_or_default();
    let attachments = about.attachments.clone().unwrap_or_default();
    let styles = about.style_brick_ids.clone().unwrap_or_default();
    spawn_task(
        &app,
        state.inner(),
        &work_id,
        &action,
        about.as_about(&attachments, &styles),
    )
}

/// What a task would send, without sending it: the prompt and the method,
/// composed by the very call that starts one, so the preview and the run
/// cannot part.
#[tauri::command]
pub fn preview_task(
    state: State<'_, AppState>,
    work_id: String,
    action: String,
    about: Option<TaskAbout>,
) -> Result<assistant::task::Composed> {
    let about = about.unwrap_or_default();
    let attachments = about.attachments.clone().unwrap_or_default();
    let styles = about.style_brick_ids.clone().unwrap_or_default();
    let conn = state.conn();
    assistant::task::compose(
        &conn,
        &work_id,
        &action,
        about.as_about(&attachments, &styles),
    )
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
    about: assistant::task::About<'_>,
) -> Result<StartedTask> {
    let runs = Arc::clone(state.runs());
    let workdir = state.assistant_dir();

    // Checked before the chat is opened: a refused duplicate must not leave an
    // empty chat behind for every impatient second click.
    let key = match about.scene_id {
        Some(scene_id) => assistant::task::scene_key(action, work_id, scene_id),
        None => assistant::task::key(action, work_id),
    };
    if runs.task_running(&key) {
        return Err(crate::error::Error::AlreadyRunning);
    }

    let (prepared, run, stream) = {
        let conn = state.conn();
        let prepared = assistant::task::prepare(&conn, work_id, action, about)?;
        let (run, stream) = assistant_run::start_as(
            &conn,
            &runs,
            &prepared.chat_id,
            &prepared.prompt,
            workdir.as_deref(),
            Some(prepared.key.clone()),
            &prepared.attachments,
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
        let outcome = spawn_task(
            app,
            state,
            &next.work_id,
            &next.action,
            assistant::task::About::default(),
        );
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
            match spawn_task(
                &app,
                inner,
                work_id,
                &action,
                assistant::task::About::default(),
            ) {
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
                // The word as it was, not the label: a journal line is a record
                // of a past event, and it reads the same tomorrow whichever
                // language the window is in then.
                .param("action", label.as_str())
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
    prompt::for_work(&conn, &work_id, &template, prompt::Context::default())
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
