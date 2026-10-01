//! Works: made, read, changed, cloned, and the catalogue that lists them.

use tauri::State;

use super::active;
use crate::actions::{self, BulkOutcome};
use crate::card;
use crate::error::Result;
use crate::profile;
use crate::score::{self, ScoredWork};
use crate::state::AppState;
use crate::trash::Entity;
use crate::work::{self, NewWork, Work, WorkFilter, WorkPatch};

#[tauri::command]
pub fn list_works(state: State<'_, AppState>, filter: Option<WorkFilter>) -> Result<Vec<Work>> {
    let conn = state.conn();
    work::list(&conn, &active(&conn)?, &filter.unwrap_or_default())
}

#[tauri::command]
pub fn get_work(state: State<'_, AppState>, id: String) -> Result<Option<Work>> {
    work::get(&state.conn(), &id)
}

#[tauri::command]
pub fn create_work(state: State<'_, AppState>, work: NewWork) -> Result<Work> {
    actions::work::create(&state.conn(), work)
}

#[tauri::command]
pub fn update_work(state: State<'_, AppState>, id: String, patch: WorkPatch) -> Result<Work> {
    actions::work::update(&state.conn(), &id, patch)
}

/// The Cover tab of a work (v0.88): the prompt written from its cover for
/// one of its doors' shapes, the scheme drawn from the same settings, and
/// what the channel's card offers the constructor. Written here, once, so
/// the tab and an agent read the same words.
#[tauri::command]
pub fn cover_view(
    state: State<'_, AppState>,
    id: String,
    format: Option<String>,
) -> Result<crate::cover::read::CoverView> {
    crate::cover::read::view(&state.conn(), &id, format.as_deref())
}

/// The frame of a work as it is copied into a generator: the still - built
/// from the cover without its words, or written whole - the loop written
/// from its settings, the negative (v0.86, v0.88).
#[tauri::command]
pub fn frame_view(state: State<'_, AppState>, id: String) -> Result<crate::cover::read::FrameView> {
    crate::cover::read::frame_view(&state.conn(), &id)
}

/// What a full recompute would change, changing nothing.
///
/// The dry run is the whole reason a mass restate is safe to offer: a profile
/// whose `derive` roles are wrong would otherwise silently rewrite the status
/// of every work in the workspace.
#[tauri::command]
pub fn status_drift(state: State<'_, AppState>) -> Result<Vec<work::status::Change>> {
    let conn = state.conn();
    let profile_id = active(&conn)?;
    let config = profile::config_for(&conn, &profile_id)?;
    work::status::drift(&conn, &config, &profile_id)
}

/// Apply what [`status_drift`] reported.
#[tauri::command]
pub fn resync_statuses(state: State<'_, AppState>) -> Result<Vec<work::status::Change>> {
    actions::work::resync_statuses(&state.conn())
}

#[tauri::command]
pub fn unpin_status(state: State<'_, AppState>, id: String) -> Result<Work> {
    actions::work::unpin_status(&state.conn(), &id)
}

#[tauri::command]
pub fn pin_tier(
    state: State<'_, AppState>,
    id: String,
    tier: String,
    reason: String,
) -> Result<Work> {
    actions::work::pin_tier(&state.conn(), &id, &tier, &reason)
}

#[tauri::command]
pub fn unpin_tier(state: State<'_, AppState>, id: String) -> Result<Work> {
    actions::work::unpin_tier(&state.conn(), &id)
}

/// Move a work to the trash. Returns the entry id, which is what an undo
/// needs. Nothing in the app deletes outright, so an undo is always
/// available and a confirmation never is.
#[tauri::command]
pub fn delete_work(state: State<'_, AppState>, id: String) -> Result<String> {
    actions::trash::discard(&state.conn(), Entity::Work, &id)
}

#[tauri::command]
pub fn delete_works(
    state: State<'_, AppState>,
    ids: Vec<String>,
) -> Result<actions::work::Discarded> {
    actions::work::discard(&state.conn(), &ids)
}

#[tauri::command]
pub fn set_works_status(
    state: State<'_, AppState>,
    work_ids: Vec<String>,
    status: String,
) -> Result<BulkOutcome> {
    actions::work::set_status(&state.conn(), &work_ids, &status)
}

/// Tags in use on works, for completing the next one. Separate from a note's
/// tags: offering "reference" while tagging a song would be the app guessing
/// at a connection nobody made.
#[tauri::command]
pub fn work_tags(state: State<'_, AppState>) -> Result<Vec<(String, i64)>> {
    let conn = state.conn();
    work::tags(&conn, &active(&conn)?)
}

#[tauri::command]
pub fn catalogue(state: State<'_, AppState>) -> Result<Vec<ScoredWork>> {
    let conn = state.conn();
    score::catalogue(&conn, &active(&conn)?)
}

/// The number beside each of a work's tabs, in one answer.
#[tauri::command]
pub fn card_counts(state: State<'_, AppState>, work_id: String) -> Result<card::Counts> {
    card::counts(&state.conn(), &work_id)
}

#[tauri::command]
pub fn clone_work(
    state: State<'_, AppState>,
    work_id: String,
    title: String,
) -> Result<crate::clone::Cloned> {
    actions::work::clone(&state.conn(), &work_id, &title)
}
