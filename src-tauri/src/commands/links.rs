//! Links between works: what a work was made from, and what was made from it.

use tauri::State;

use crate::actions;
use crate::error::Result;
use crate::link::{self, Links, NewLink};
use crate::state::AppState;
use crate::work::Work;

#[tauri::command]
pub fn list_links(state: State<'_, AppState>, work_id: String) -> Result<Links> {
    link::for_work(&state.conn(), &work_id)
}

#[tauri::command]
pub fn create_link(state: State<'_, AppState>, link: NewLink) -> Result<link::Link> {
    actions::link::create(&state.conn(), link)
}

#[tauri::command]
pub fn delete_link(state: State<'_, AppState>, id: String) -> Result<()> {
    actions::link::delete(&state.conn(), &id)
}

/// Make a work from another: a video from a song.
#[tauri::command]
pub fn derive_work(
    state: State<'_, AppState>,
    source_id: String,
    kind: String,
    title: Option<String>,
) -> Result<Work> {
    actions::work::derive(&state.conn(), &source_id, &kind, title.as_deref())
}
