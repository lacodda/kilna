use tauri::State;

use crate::assistant::{self, Chat, Message, NewChat, Transcript, cli, prompt};
use crate::collection::{self, Collection, CollectionPatch, NewCollection};
use crate::error::{Error, Result};
use crate::exchange::backup;
use crate::exchange::export::{self, ExportReport};
use crate::exchange::import::{self, ImportReport};
use crate::journal::{self, Entry, Record};
use crate::note::{self, NewNote, Note, NoteFilter, NotePatch};
use crate::plugin::{self, manifest::Plugin, manifest::Target};
use crate::profile::{self, Profile, Workspace};
use crate::release::{self, NewRelease, Release, ReleasePatch, ScheduledRelease, Scheduling};
use crate::score::{self, NewScore, Score, ScoredWork};
use crate::search::{self, Hit};
use crate::state::AppState;
use crate::trash::{self, Deletion};
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
    let conn = state.conn();
    let profile_id = active_profile_id(&conn)?;
    let created = work::create(&conn, &profile_id, work)?;

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
    let conn = state.conn();
    let profile_id = active_profile_id(&conn)?;
    let before = work::get(&conn, &id)?;
    let updated = work::update(&conn, &id, patch)?;

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
    let conn = state.conn();
    let profile_id = active_profile_id(&conn)?;
    let config = profile::config_for(&conn, &profile_id)?;
    let changes = work::status::resync(&conn, &config, &profile_id)?;

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
    let conn = state.conn();
    let profile_id = active_profile_id(&conn)?;
    let config = profile::config_for(&conn, &profile_id)?;

    if let Some(change) = work::status::unpin(&conn, &config, &id)? {
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

#[tauri::command]
pub fn delete_work(state: State<'_, AppState>, id: String) -> Result<String> {
    let mut conn = state.conn();
    let profile_id = active_profile_id(&conn)?;
    // Read the title while the row is still there. Afterwards there is nothing
    // left to ask, and an entry saying a work was deleted without saying which
    // one is worse than no entry at all.
    let title = journal::work_title(&conn, &id);
    let entry = trash::discard(&mut conn, trash::Entity::Work, &id)?;

    journal::record(
        &conn,
        &profile_id,
        Record::new("work.deleted")
            .param("title", title.unwrap_or_else(|| id.clone()))
            .about("work", id),
    );

    Ok(entry)
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
    let created = version::create(&mut conn, &work_id, version)?;

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
    let entry_id = trash::discard(&mut conn, entity, id)?;

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
    let conn = state.conn();
    let profile_id = active_profile_id(&conn)?;
    note::create(&conn, &profile_id, note)
}

#[tauri::command]
pub fn update_note(state: State<'_, AppState>, id: String, patch: NotePatch) -> Result<Note> {
    let conn = state.conn();
    note::update(&conn, &id, patch)
}

#[tauri::command]
pub fn delete_note(state: State<'_, AppState>, id: String) -> Result<String> {
    discard_and_record(&state, trash::Entity::Note, &id)
}

#[tauri::command]
pub fn list_tags(state: State<'_, AppState>) -> Result<Vec<(String, i64)>> {
    let conn = state.conn();
    let profile_id = active_profile_id(&conn)?;
    note::tags(&conn, &profile_id)
}

#[tauri::command]
pub fn score_work(state: State<'_, AppState>, work_id: String, score: NewScore) -> Result<Score> {
    let conn = state.conn();
    let profile_id = active_profile_id(&conn)?;
    let created = score::create(&conn, &work_id, score)?;

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
    let conn = state.conn();
    let profile_id = active_profile_id(&conn)?;
    let created = release::create(&conn, release)?;

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
    let conn = state.conn();
    let profile_id = active_profile_id(&conn)?;
    let before = release::get(&conn, &id)?;
    let updated = release::update(&conn, &id, patch)?;

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
    let outcome = release::schedule(&mut conn, &id, &slot)?;

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

    // Losing a slot is the one thing here that happens *to* someone rather than
    // because of them: the toast that says so is gone in seconds, and this is
    // where they find out afterwards what moved and when.
    if let Some(displaced) = &outcome.displaced {
        journal::record(
            &conn,
            &profile_id,
            Record::new("release.displaced")
                .param(
                    "title",
                    journal::work_title(&conn, &displaced.work_id).unwrap_or_default(),
                )
                .param("slot", slot)
                .about("work", displaced.work_id.clone())
                // Keyed on the release, not on the release and the date: a work
                // that keeps being pushed out of the calendar is one situation
                // told once with a count, not a new line every time somebody
                // stronger arrives. The date in the line is the latest one,
                // which is the one worth acting on.
                .deduped(format!("displaced:{}", displaced.id))
                .warn(),
        );
    }

    restate(&conn, &profile_id, &outcome.release.work_id);
    // The work that lost the slot may no longer be scheduled at all.
    if let Some(displaced) = &outcome.displaced {
        restate(&conn, &profile_id, &displaced.work_id);
    }

    Ok(outcome)
}

/// Settle a date so the contest leaves it alone, or hand it back.
#[tauri::command]
pub fn set_slot_pin(state: State<'_, AppState>, id: String, pinned: bool) -> Result<Release> {
    let conn = state.conn();
    let profile_id = active_profile_id(&conn)?;
    let updated = release::set_slot_pin(&conn, &id, pinned)?;

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
    let conn = state.conn();
    let profile_id = active_profile_id(&conn)?;
    let release = release::unschedule(&conn, &id)?;
    restate(&conn, &profile_id, &release.work_id);
    Ok(release)
}

/// The end of the circle: something actually went out.
#[tauri::command]
pub fn mark_released(
    state: State<'_, AppState>,
    id: String,
    url: Option<String>,
) -> Result<Release> {
    let conn = state.conn();
    let profile_id = active_profile_id(&conn)?;
    let released = release::mark_released(&conn, &id, url)?;

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
pub fn releases_for_work(state: State<'_, AppState>, work_id: String) -> Result<Vec<Release>> {
    let conn = state.conn();
    release::for_work(&conn, &work_id)
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
    let conn = state.conn();
    let profile_id = active_profile_id(&conn)?;
    collection::create(&conn, &profile_id, collection)
}

#[tauri::command]
pub fn update_collection(
    state: State<'_, AppState>,
    id: String,
    patch: CollectionPatch,
) -> Result<Collection> {
    let conn = state.conn();
    collection::update(&conn, &id, patch)
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

    trash::restore(&mut conn, &id)?;

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
    trash::purge(&mut conn, &id)
}

/// Empty the active profile's trash. Returns how many entries went.
#[tauri::command]
pub fn empty_trash(state: State<'_, AppState>) -> Result<usize> {
    let conn = state.conn();
    let profile_id = active_profile_id(&conn)?;
    trash::empty(&conn, &profile_id)
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

#[tauri::command]
pub fn list_chats(state: State<'_, AppState>, work_id: Option<String>) -> Result<Vec<Chat>> {
    let conn = state.conn();
    let profile_id = active_profile_id(&conn)?;
    assistant::list(&conn, &profile_id, work_id.as_deref())
}

#[tauri::command]
pub fn create_chat(state: State<'_, AppState>, chat: NewChat) -> Result<Chat> {
    let conn = state.conn();
    let profile_id = active_profile_id(&conn)?;
    assistant::create(&conn, &profile_id, chat)
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
/// This blocks for as long as the CLI takes. Tauri runs commands off the UI
/// thread, so the window stays responsive; the panel shows its own pending
/// state.
#[tauri::command]
pub fn ask_assistant(
    state: State<'_, AppState>,
    chat_id: String,
    prompt: String,
) -> Result<Message> {
    let mut conn = state.conn();
    assistant::ask(&mut conn, &chat_id, &prompt)
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
    collection::set_contents(&mut conn, &id, &work_ids)
}
