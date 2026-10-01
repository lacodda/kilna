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

/// Attach a pasted picture to a work: a candidate for its cover, taken
/// straight from a generator's page. The window sends the bytes, as it does
/// for a scene's frame.
#[tauri::command(async)]
pub fn paste_asset(
    state: State<'_, AppState>,
    bytes: Vec<u8>,
    name: String,
    asset: NewAsset,
) -> Result<Asset> {
    let media = state.media_dir()?;
    actions::asset::paste(&state.conn(), &media, &bytes, &name, asset)
}

/// Make a picture the work's cover - the final one, the release's preview.
#[tauri::command]
pub fn choose_cover(state: State<'_, AppState>, id: String) -> Result<Asset> {
    actions::asset::choose_cover(&state.conn(), &id)
}

/// The bytes of a file of the workspace, for the window to draw on: a cover
/// exported with the channel's mark laid over it is composed in the window,
/// and a picture fetched from the asset protocol would taint the canvas.
#[tauri::command(async)]
pub fn asset_bytes(state: State<'_, AppState>, id: String) -> Result<tauri::ipc::Response> {
    let found = asset::get(&state.conn(), &id)?
        .ok_or_else(|| crate::error::Error::not_found("asset", &id))?;
    let bytes = std::fs::read(&found.path).map_err(|cause| {
        crate::log::warn("asset", &format!("could not read {}: {cause}", found.path));
        crate::error::Error::refused("asset.fileMissing").param("path", found.path.clone())
    })?;
    Ok(tauri::ipc::Response::new(bytes))
}

/// Write a picture the window composed to a place the person chose: a cover
/// with the channel's mark laid over it, ready to upload. Not an asset - it
/// is a copy for somewhere else, made again whenever it is needed.
#[tauri::command(async)]
pub fn save_picture(path: String, bytes: Vec<u8>) -> Result<()> {
    std::fs::write(&path, bytes).map_err(|cause| {
        crate::error::Error::refused("asset.saveFailed")
            .param("path", path)
            .param("cause", cause.to_string())
    })
}
