//! The trash, and undo: taking back what was deleted, and what was done last.

use tauri::State;

use super::active;
use crate::actions;
use crate::error::Result;
use crate::state::AppState;
use crate::trash::{self, Deletion};
use crate::undo;

/// Everything in the active profile's trash, newest first.
#[tauri::command]
pub fn list_deletions(state: State<'_, AppState>) -> Result<Vec<Deletion>> {
    let conn = state.conn();
    trash::list(&conn, &active(&conn)?)
}

#[tauri::command]
pub fn restore_deletion(state: State<'_, AppState>, id: String) -> Result<()> {
    actions::trash::restore(&state.conn(), &id)
}

#[tauri::command]
pub fn purge_deletion(state: State<'_, AppState>, id: String) -> Result<()> {
    actions::trash::purge(&state.conn(), &id)
}

#[tauri::command]
pub fn empty_trash(state: State<'_, AppState>) -> Result<usize> {
    actions::trash::empty(&state.conn())
}

/// What pressing undo would take back, if anything.
///
/// Read fresh rather than remembered by the window: between the last action
/// and the keystroke, a sweep or a second window may have written, and an
/// offer built from a stale memory would name the wrong thing.
#[tauri::command]
pub fn last_undoable(state: State<'_, AppState>) -> Result<Option<undo::Undoable>> {
    undo::last(&state.conn())
}

#[tauri::command]
pub fn undo_last(state: State<'_, AppState>, operation: String) -> Result<undo::Undoable> {
    actions::trash::undo(&state.conn(), &operation)
}
