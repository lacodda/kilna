//! The focus board: findings a person dismissed, and notes pinned to it.

use tauri::State;

use super::active;
use crate::actions;
use crate::error::Result;
use crate::focus::{self, Dismissal, DismissalKey, FocusNote, FocusNotePatch, NewFocusNote};
use crate::state::AppState;

#[tauri::command]
pub fn dismissed_findings(state: State<'_, AppState>) -> Result<Vec<Dismissal>> {
    let conn = state.conn();
    focus::dismissals(&conn, &active(&conn)?)
}

#[tauri::command]
pub fn dismiss_finding(state: State<'_, AppState>, key: DismissalKey) -> Result<Dismissal> {
    actions::focus::dismiss(&state.conn(), &key)
}

#[tauri::command]
pub fn restore_finding(state: State<'_, AppState>, key: DismissalKey) -> Result<()> {
    actions::focus::restore(&state.conn(), &key)
}

#[tauri::command]
pub fn list_focus_notes(state: State<'_, AppState>) -> Result<Vec<FocusNote>> {
    let conn = state.conn();
    focus::notes(&conn, &active(&conn)?)
}

#[tauri::command]
pub fn create_focus_note(state: State<'_, AppState>, note: NewFocusNote) -> Result<FocusNote> {
    actions::focus::create_note(&state.conn(), note)
}

#[tauri::command]
pub fn update_focus_note(
    state: State<'_, AppState>,
    id: String,
    patch: FocusNotePatch,
) -> Result<FocusNote> {
    actions::focus::update_note(&state.conn(), &id, patch)
}

#[tauri::command]
pub fn reorder_focus_notes(state: State<'_, AppState>, order: Vec<String>) -> Result<()> {
    actions::focus::reorder_notes(&state.conn(), &order)
}

#[tauri::command]
pub fn delete_focus_note(state: State<'_, AppState>, id: String) -> Result<()> {
    actions::focus::delete_note(&state.conn(), &id)
}
