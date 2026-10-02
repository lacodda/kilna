//! Links between works: what a work was made from, and what was made from it.

use tauri::State;

use crate::actions;
use crate::error::Result;
use crate::link::{self, Links, NewLink};
use crate::state::AppState;

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

/// Make a work from another: a clip, an audio release or a short from a
/// song. `locale` is the window's language, for a profile that names what it
/// makes per language; `door` is where it goes out, the kind's first when
/// none is asked for.
#[tauri::command]
pub fn derive_work(
    state: State<'_, AppState>,
    source_id: String,
    kind: String,
    title: Option<String>,
    locale: Option<String>,
    door: Option<String>,
) -> Result<actions::work::Made> {
    actions::work::derive(
        &state.conn(),
        &source_id,
        &kind,
        title.as_deref(),
        locale.as_deref(),
        door.as_deref(),
    )
}

/// What was made from a work, directly or through what was made from it,
/// and the release its status stands on (v0.86).
#[tauri::command]
pub fn list_publications(
    state: State<'_, AppState>,
    work_id: String,
) -> Result<crate::publication::Publications> {
    let conn = state.conn();
    let profile_id = actions::active_profile_id(&conn)?;
    let config = crate::profile::config_for(&conn, &profile_id)?;
    crate::publication::of(&conn, &config, &work_id)
}
