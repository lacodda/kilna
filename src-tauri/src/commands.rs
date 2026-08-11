use tauri::State;

use crate::error::{Error, Result};
use crate::note::{self, NewNote, Note, NoteFilter, NotePatch};
use crate::profile::{self, Profile, Workspace};
use crate::state::AppState;
use crate::work::version::{self, NewVersion, Version, VersionSummary};
use crate::work::{self, NewWork, Work, WorkFilter, WorkPatch};

/// Everything the status screen needs in one round trip.
#[tauri::command]
pub fn get_workspace(state: State<'_, AppState>) -> Result<Workspace> {
    let conn = state.conn();
    profile::workspace(&conn)
}

#[tauri::command]
pub fn list_profiles(state: State<'_, AppState>) -> Result<Vec<Profile>> {
    let conn = state.conn();
    profile::list(&conn)
}

#[tauri::command]
pub fn activate_profile(state: State<'_, AppState>, id: String) -> Result<()> {
    let mut conn = state.conn();
    profile::activate(&mut conn, &id)
}

/// The id of the active profile, or an error the frontend can show.
///
/// Every work and note command is scoped to it: the frontend never has to carry
/// the profile id around, and a request cannot land in the wrong profile.
fn active_profile_id(conn: &rusqlite::Connection) -> Result<String> {
    profile::active(conn)?
        .map(|profile| profile.id)
        .ok_or_else(|| Error::Other("no active profile".into()))
}

#[tauri::command]
pub fn list_works(state: State<'_, AppState>, filter: Option<WorkFilter>) -> Result<Vec<Work>> {
    let conn = state.conn();
    let profile_id = active_profile_id(&conn)?;
    work::list(&conn, &profile_id, &filter.unwrap_or_default())
}

#[tauri::command]
pub fn get_work(state: State<'_, AppState>, id: String) -> Result<Option<Work>> {
    let conn = state.conn();
    work::get(&conn, &id)
}

#[tauri::command]
pub fn create_work(state: State<'_, AppState>, work: NewWork) -> Result<Work> {
    let conn = state.conn();
    let profile_id = active_profile_id(&conn)?;
    work::create(&conn, &profile_id, work)
}

#[tauri::command]
pub fn update_work(state: State<'_, AppState>, id: String, patch: WorkPatch) -> Result<Work> {
    let conn = state.conn();
    work::update(&conn, &id, patch)
}

#[tauri::command]
pub fn delete_work(state: State<'_, AppState>, id: String) -> Result<()> {
    let conn = state.conn();
    work::delete(&conn, &id)
}

#[tauri::command]
pub fn list_versions(state: State<'_, AppState>, work_id: String) -> Result<Vec<VersionSummary>> {
    let conn = state.conn();
    version::list(&conn, &work_id)
}

#[tauri::command]
pub fn get_version(state: State<'_, AppState>, id: String) -> Result<Option<Version>> {
    let conn = state.conn();
    version::get(&conn, &id)
}

#[tauri::command]
pub fn create_version(
    state: State<'_, AppState>,
    work_id: String,
    version: NewVersion,
) -> Result<Version> {
    let mut conn = state.conn();
    version::create(&mut conn, &work_id, version)
}

#[tauri::command]
pub fn set_current_version(
    state: State<'_, AppState>,
    work_id: String,
    version_id: String,
) -> Result<()> {
    let conn = state.conn();
    version::set_current(&conn, &work_id, &version_id)
}

#[tauri::command]
pub fn delete_version(state: State<'_, AppState>, id: String) -> Result<()> {
    let mut conn = state.conn();
    version::delete(&mut conn, &id)
}

#[tauri::command]
pub fn list_notes(state: State<'_, AppState>, filter: Option<NoteFilter>) -> Result<Vec<Note>> {
    let conn = state.conn();
    let profile_id = active_profile_id(&conn)?;
    note::list(&conn, &profile_id, &filter.unwrap_or_default())
}

#[tauri::command]
pub fn create_note(state: State<'_, AppState>, note: NewNote) -> Result<Note> {
    let conn = state.conn();
    let profile_id = active_profile_id(&conn)?;
    note::create(&conn, &profile_id, note)
}

#[tauri::command]
pub fn update_note(state: State<'_, AppState>, id: String, patch: NotePatch) -> Result<Note> {
    let conn = state.conn();
    note::update(&conn, &id, patch)
}

#[tauri::command]
pub fn delete_note(state: State<'_, AppState>, id: String) -> Result<()> {
    let conn = state.conn();
    note::delete(&conn, &id)
}

#[tauri::command]
pub fn list_tags(state: State<'_, AppState>) -> Result<Vec<(String, i64)>> {
    let conn = state.conn();
    let profile_id = active_profile_id(&conn)?;
    note::tags(&conn, &profile_id)
}
