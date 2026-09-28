//! The style dictionary: bricks and the pictures they were described from.

use tauri::State;

use super::active;
use crate::actions;
use crate::asset;
use crate::error::Result;
use crate::state::AppState;
use crate::style_brick;
use crate::trash::Entity;

/// The workspace's style dictionary, in the profile's order of types.
#[tauri::command]
pub fn list_style_bricks(
    state: State<'_, AppState>,
    filter: Option<style_brick::StyleBrickFilter>,
) -> Result<Vec<style_brick::StyleBrick>> {
    let conn = state.conn();
    style_brick::list(&conn, &active(&conn)?, &filter.unwrap_or_default())
}

/// How many bricks stand under each type, for the counts beside the filter.
#[tauri::command]
pub fn style_brick_counts(state: State<'_, AppState>) -> Result<Vec<(String, i64)>> {
    let conn = state.conn();
    style_brick::counts(&conn, &active(&conn)?)
}

/// The pictures a brick was described from.
#[tauri::command]
pub fn style_brick_references(state: State<'_, AppState>, id: String) -> Result<Vec<asset::Asset>> {
    asset::for_style_brick(&state.conn(), &id)
}

#[tauri::command]
pub fn create_style_brick(
    state: State<'_, AppState>,
    brick: style_brick::NewStyleBrick,
) -> Result<style_brick::StyleBrick> {
    actions::style::create(&state.conn(), brick)
}

#[tauri::command]
pub fn update_style_brick(
    state: State<'_, AppState>,
    id: String,
    patch: style_brick::StyleBrickPatch,
) -> Result<style_brick::StyleBrick> {
    actions::style::update(&state.conn(), &id, patch)
}

/// A reference picture pasted straight onto a brick.
#[tauri::command(async)]
pub fn paste_style_reference(
    state: State<'_, AppState>,
    id: String,
    bytes: Vec<u8>,
    name: String,
) -> Result<asset::Asset> {
    let media = state.media_dir()?;
    actions::style::paste_reference(&state.conn(), &media, &id, &bytes, &name)
}

/// Move a brick to the trash, with the pictures it was described from - the
/// one road every other deletion takes.
#[tauri::command]
pub fn delete_style_brick(state: State<'_, AppState>, id: String) -> Result<String> {
    actions::trash::discard(&state.conn(), Entity::Style, &id)
}
