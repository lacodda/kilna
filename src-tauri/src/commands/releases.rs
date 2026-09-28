//! Releases: what goes out where and when, and the calendar they make.

use std::collections::BTreeMap;

use tauri::State;

use super::active;
use crate::actions::{self, BulkOutcome, release::GeneratedBatch};
use crate::error::Result;
use crate::layout;
use crate::release::{self, NewRelease, Release, ReleasePatch, ScheduledRelease, Scheduling};
use crate::release_meta;
use crate::state::AppState;
use crate::trash::Entity;

#[tauri::command]
pub fn create_release(state: State<'_, AppState>, release: NewRelease) -> Result<Release> {
    actions::release::create(&state.conn(), release)
}

#[tauri::command]
pub fn update_release(
    state: State<'_, AppState>,
    id: String,
    patch: ReleasePatch,
) -> Result<Release> {
    actions::release::update(&state.conn(), &id, patch)
}

#[tauri::command]
pub fn delete_release(state: State<'_, AppState>, id: String) -> Result<String> {
    actions::trash::discard(&state.conn(), Entity::Release, &id)
}

/// What a release says about itself, field by field: the fields of its kind,
/// so a clip is asked for a title, a description, tags and a pinned comment,
/// and a beta read is asked for nothing at all.
#[tauri::command]
pub fn release_fields(state: State<'_, AppState>, id: String) -> Result<Vec<release_meta::Field>> {
    release_meta::fields(&state.conn(), &id)
}

#[tauri::command]
pub fn set_release_fields(
    state: State<'_, AppState>,
    id: String,
    values: BTreeMap<String, String>,
) -> Result<Release> {
    actions::release::set_fields(&state.conn(), &id, &values)
}

/// What the profile would write in a release's fields, without writing it:
/// shown before it lands, because a generated description replaces one
/// someone may have edited by hand.
#[tauri::command]
pub fn preview_release_fields(
    state: State<'_, AppState>,
    id: String,
) -> Result<release_meta::Generated> {
    release_meta::generate(&state.conn(), &id)
}

#[tauri::command]
pub fn generate_release_fields(
    state: State<'_, AppState>,
    id: String,
) -> Result<release_meta::Generated> {
    actions::release::generate_fields(&state.conn(), &id)
}

#[tauri::command]
pub fn generate_release_fields_batch(
    state: State<'_, AppState>,
    ids: Vec<String>,
) -> Result<GeneratedBatch> {
    actions::release::generate_fields_batch(&state.conn(), &ids)
}

/// The dry run of a claim: how `schedule_release` would end, without moving
/// anything.
#[tauri::command]
pub fn preview_schedule(
    state: State<'_, AppState>,
    id: String,
    slot: String,
) -> Result<release::SlotPreview> {
    release::preview(&state.conn(), &id, &slot)
}

#[tauri::command]
pub fn warn_unready_releases(state: State<'_, AppState>, today: String) -> Result<usize> {
    actions::release::warn_unready(&state.conn(), &today)
}

#[tauri::command]
pub fn schedule_release(
    state: State<'_, AppState>,
    id: String,
    slot: String,
) -> Result<Scheduling> {
    actions::release::schedule(&state.conn(), &id, &slot)
}

#[tauri::command]
pub fn set_slot_pin(state: State<'_, AppState>, id: String, pinned: bool) -> Result<Release> {
    actions::release::set_slot_pin(&state.conn(), &id, pinned)
}

#[tauri::command]
pub fn unschedule_release(state: State<'_, AppState>, id: String) -> Result<Release> {
    actions::release::unschedule(&state.conn(), &id)
}

#[tauri::command]
pub fn unschedule_works(state: State<'_, AppState>, work_ids: Vec<String>) -> Result<BulkOutcome> {
    actions::release::unschedule_works(&state.conn(), &work_ids)
}

#[tauri::command]
pub fn mark_released(
    state: State<'_, AppState>,
    id: String,
    url: Option<String>,
    at: Option<String>,
) -> Result<Release> {
    actions::release::mark_released(&state.conn(), &id, url, at)
}

#[tauri::command]
pub fn unmark_released(state: State<'_, AppState>, id: String) -> Result<Release> {
    actions::release::unmark_released(&state.conn(), &id)
}

#[tauri::command]
pub fn calendar(state: State<'_, AppState>) -> Result<Vec<ScheduledRelease>> {
    let conn = state.conn();
    release::calendar(&conn, &active(&conn)?)
}

/// Where the queue would land, laid out to the profile's rhythm. Nothing is
/// written: the person approves the plan, or nothing happens.
#[tauri::command]
pub fn plan_layout(state: State<'_, AppState>, today: String) -> Result<Vec<layout::Placement>> {
    let conn = state.conn();
    layout::plan(&conn, &active(&conn)?, &today)
}

#[tauri::command]
pub fn apply_layout(
    state: State<'_, AppState>,
    placements: Vec<layout::Placement>,
) -> Result<usize> {
    actions::release::apply_layout(&state.conn(), &placements)
}

#[tauri::command]
pub fn release_queue(state: State<'_, AppState>) -> Result<Vec<ScheduledRelease>> {
    let conn = state.conn();
    release::queue(&conn, &active(&conn)?)
}

#[tauri::command]
pub fn releases_for_work(
    state: State<'_, AppState>,
    work_id: String,
) -> Result<Vec<ScheduledRelease>> {
    let conn = state.conn();
    release::for_work(&conn, &active(&conn)?, &work_id)
}
