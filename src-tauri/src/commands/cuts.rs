//! The stretches a short is cut from.

use tauri::State;

use crate::actions;
use crate::cut::{self, Cut, CutPatch, NewCut, Shot};
use crate::error::Result;
use crate::state::AppState;
use crate::trash::Entity;

/// The stretches a short is spliced from, in order.
#[tauri::command]
pub fn list_cuts(state: State<'_, AppState>, work_id: String) -> Result<Vec<Cut>> {
    cut::for_work(&state.conn(), &work_id)
}

/// What has been cut out of this work — the question a donor's card asks.
#[tauri::command]
pub fn list_cuts_from(state: State<'_, AppState>, source_id: String) -> Result<Vec<Cut>> {
    cut::from_source(&state.conn(), &source_id)
}

#[tauri::command]
pub fn create_cut(state: State<'_, AppState>, cut: NewCut) -> Result<Cut> {
    actions::cut::create(&state.conn(), cut)
}

#[tauri::command]
pub fn update_cut(state: State<'_, AppState>, id: String, patch: CutPatch) -> Result<Cut> {
    actions::cut::update(&state.conn(), &id, patch)
}

#[tauri::command]
pub fn reorder_cuts(
    state: State<'_, AppState>,
    work_id: String,
    ids: Vec<String>,
) -> Result<Vec<Cut>> {
    actions::cut::reorder(&state.conn(), &work_id, &ids)
}

#[tauri::command]
pub fn delete_cut(state: State<'_, AppState>, id: String) -> Result<String> {
    actions::trash::discard(&state.conn(), Entity::Cut, &id)
}

/// What a short is, told to something that can cut video: the stretches in
/// order, each beside the donor's video on disk. The core does not run ffmpeg
/// (decision of 2026-09-11).
#[tauri::command]
pub fn cut_shot_list(state: State<'_, AppState>, work_id: String) -> Result<Vec<Shot>> {
    cut::shot_list(&state.conn(), &work_id)
}
