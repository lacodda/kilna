//! Collections: an album, a book, a season - one level, no nesting.

use tauri::State;

use super::active;
use crate::actions;
use crate::collection::{self, Collection, CollectionPatch, NewCollection};
use crate::error::Result;
use crate::state::AppState;
use crate::trash::Entity;

#[tauri::command]
pub fn list_collections(state: State<'_, AppState>) -> Result<Vec<Collection>> {
    let conn = state.conn();
    collection::list(&conn, &active(&conn)?)
}

#[tauri::command]
pub fn create_collection(
    state: State<'_, AppState>,
    collection: NewCollection,
) -> Result<Collection> {
    actions::collection::create(&state.conn(), collection)
}

#[tauri::command]
pub fn update_collection(
    state: State<'_, AppState>,
    id: String,
    patch: CollectionPatch,
) -> Result<Collection> {
    actions::collection::update(&state.conn(), &id, patch)
}

#[tauri::command]
pub fn delete_collection(state: State<'_, AppState>, id: String) -> Result<String> {
    actions::trash::discard(&state.conn(), Entity::Collection, &id)
}

#[tauri::command]
pub fn set_collection_contents(
    state: State<'_, AppState>,
    id: String,
    work_ids: Vec<String>,
) -> Result<()> {
    actions::collection::set_contents(&state.conn(), &id, &work_ids)
}
