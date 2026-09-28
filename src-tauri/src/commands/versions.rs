//! A work's versions: every body it was written in.

use tauri::State;

use crate::actions;
use crate::error::Result;
use crate::state::AppState;
use crate::trash::Entity;
use crate::work::Work;
use crate::work::version::{self, NewVersion, Version, VersionSummary};

#[tauri::command]
pub fn list_versions(state: State<'_, AppState>, work_id: String) -> Result<Vec<VersionSummary>> {
    version::list(&state.conn(), &work_id)
}

#[tauri::command]
pub fn get_version(state: State<'_, AppState>, id: String) -> Result<Option<Version>> {
    version::get(&state.conn(), &id)
}

#[tauri::command]
pub fn create_version(
    state: State<'_, AppState>,
    work_id: String,
    version: NewVersion,
) -> Result<Version> {
    actions::version::create(&state.conn(), &work_id, version)
}

#[tauri::command]
pub fn update_version_body(
    state: State<'_, AppState>,
    id: String,
    body: String,
) -> Result<Version> {
    actions::version::edit(&state.conn(), &id, &body)
}

#[tauri::command]
pub fn set_current_version(
    state: State<'_, AppState>,
    work_id: String,
    version_id: String,
) -> Result<Work> {
    actions::work::set_current_version(&state.conn(), &work_id, &version_id)
}

#[tauri::command]
pub fn delete_version(state: State<'_, AppState>, id: String) -> Result<String> {
    actions::trash::discard(&state.conn(), Entity::Version, &id)
}
