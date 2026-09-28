//! Files copied into the workspace: covers, clips, references.

use tauri::State;

use super::active;
use crate::actions;
use crate::asset::{self, Asset, NewAsset};
use crate::error::Result;
use crate::state::AppState;

#[tauri::command(async)]
pub fn attach_asset(state: State<'_, AppState>, source: String, asset: NewAsset) -> Result<Asset> {
    let media = state.media_dir()?;
    actions::asset::attach(&state.conn(), &media, &source, asset)
}

/// The files attached to a work, oldest first.
#[tauri::command]
pub fn list_work_assets(state: State<'_, AppState>, work_id: String) -> Result<Vec<Asset>> {
    asset::for_work(&state.conn(), &work_id)
}

/// The files attached to a release, oldest first.
#[tauri::command]
pub fn list_release_assets(state: State<'_, AppState>, release_id: String) -> Result<Vec<Asset>> {
    asset::for_release(&state.conn(), &release_id)
}

/// The cover of every work that has one, by work id — one read for a
/// catalogue of two hundred rows.
#[tauri::command]
pub fn list_covers(state: State<'_, AppState>) -> Result<Vec<(String, String)>> {
    let conn = state.conn();
    asset::covers_of(&conn, &active(&conn)?)
}

#[tauri::command]
pub fn detach_asset(state: State<'_, AppState>, id: String) -> Result<()> {
    actions::asset::detach(&state.conn(), &id)
}
