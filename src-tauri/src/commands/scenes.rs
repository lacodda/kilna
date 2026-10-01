//! A storyboard: its scenes, what they are about, and the frames drawn for
//! them.

use tauri::State;

use crate::actions;
use crate::error::Result;
use crate::scene::{self, NewScene, Scene, ScenePatch};
use crate::scene_frame::{self, SceneFrame};
use crate::scene_note::{self, SceneNote};
use crate::state::AppState;
use crate::trash::Entity;

/// The storyboard of a work, in order.
#[tauri::command]
pub fn list_scenes(state: State<'_, AppState>, work_id: String) -> Result<Vec<Scene>> {
    scene::for_work(&state.conn(), &work_id)
}

#[tauri::command]
pub fn create_scene(state: State<'_, AppState>, scene: NewScene) -> Result<Scene> {
    actions::scene::create(&state.conn(), scene)
}

#[tauri::command]
pub fn update_scene(state: State<'_, AppState>, id: String, patch: ScenePatch) -> Result<Scene> {
    actions::scene::update(&state.conn(), &id, patch)
}

#[tauri::command]
pub fn delete_scene(state: State<'_, AppState>, id: String) -> Result<String> {
    actions::trash::discard(&state.conn(), Entity::Scene, &id)
}

#[tauri::command]
pub fn frame_scenes(
    state: State<'_, AppState>,
    work_id: String,
    role: String,
) -> Result<Vec<Scene>> {
    actions::scene::frame(&state.conn(), &work_id, &role)
}

#[tauri::command]
pub fn time_scenes(state: State<'_, AppState>, work_id: String) -> Result<Vec<Scene>> {
    actions::scene::time(&state.conn(), &work_id)
}

#[tauri::command]
pub fn renumber_scenes(
    state: State<'_, AppState>,
    work_id: String,
    ids: Vec<String>,
) -> Result<Vec<Scene>> {
    actions::scene::renumber(&state.conn(), &work_id, &ids)
}

/// What every scene of a board is about: the people in it, the places.
#[tauri::command]
pub fn list_scene_notes(state: State<'_, AppState>, work_id: String) -> Result<Vec<SceneNote>> {
    scene_note::for_work(&state.conn(), &work_id)
}

#[tauri::command]
pub fn attach_scene_note(
    state: State<'_, AppState>,
    scene_id: String,
    note_id: String,
) -> Result<SceneNote> {
    actions::scene::attach_note(&state.conn(), &scene_id, &note_id)
}

#[tauri::command]
pub fn detach_scene_note(
    state: State<'_, AppState>,
    scene_id: String,
    note_id: String,
) -> Result<()> {
    actions::scene::detach_note(&state.conn(), &scene_id, &note_id)
}

/// The frames of every scene of a board, in order — one read for a
/// storyboard of fifty scenes.
#[tauri::command]
pub fn list_scene_frames(state: State<'_, AppState>, work_id: String) -> Result<Vec<SceneFrame>> {
    scene_frame::for_work(&state.conn(), &work_id)
}

#[tauri::command(async)]
pub fn attach_scene_frame(
    state: State<'_, AppState>,
    scene_id: String,
    kind: String,
    source: String,
) -> Result<SceneFrame> {
    let media = state.media_dir()?;
    actions::scene::attach_frame(&state.conn(), &media, &scene_id, &kind, &source)
}

/// Hang a pasted picture on a scene. The window sends the bytes rather than
/// writing a file itself, so it needs no filesystem permissions for a picture
/// on its way into a directory this process already owns.
#[tauri::command(async)]
pub fn paste_scene_frame(
    state: State<'_, AppState>,
    scene_id: String,
    kind: String,
    bytes: Vec<u8>,
    name: String,
) -> Result<SceneFrame> {
    let media = state.media_dir()?;
    actions::scene::paste_frame(&state.conn(), &media, &scene_id, &kind, &bytes, &name)
}

/// A scene's built frame (v0.88): its still written around the picture
/// block in the clip's style, and the scheme of where its hero stands.
#[tauri::command]
pub fn scene_frame_view(
    state: State<'_, AppState>,
    id: String,
) -> Result<crate::cover::read::SceneFrameView> {
    crate::cover::read::scene_view(&state.conn(), &id)
}

#[tauri::command]
pub fn detach_scene_frame(state: State<'_, AppState>, id: String) -> Result<()> {
    actions::scene::detach_frame(&state.conn(), &id)
}

#[tauri::command]
pub fn select_scene_frame(state: State<'_, AppState>, id: String) -> Result<SceneFrame> {
    actions::scene::select_frame(&state.conn(), &id)
}

#[tauri::command]
pub fn clear_scene_frame(state: State<'_, AppState>, scene_id: String, kind: String) -> Result<()> {
    actions::scene::clear_frame(&state.conn(), &scene_id, &kind)
}

#[tauri::command]
pub fn reorder_scene_frames(
    state: State<'_, AppState>,
    scene_id: String,
    kind: String,
    ids: Vec<String>,
) -> Result<Vec<SceneFrame>> {
    actions::scene::reorder_frames(&state.conn(), &scene_id, &kind, &ids)
}
