//! The history: what happened, newest first.

use tauri::State;

use super::active;
use crate::error::Result;
use crate::journal::{self, Entry};
use crate::state::AppState;

/// The profile's history, newest first.
#[tauri::command]
pub fn list_journal(state: State<'_, AppState>) -> Result<Vec<Entry>> {
    let conn = state.conn();
    journal::list(&conn, &active(&conn)?)
}

/// One work's own history — the card's History tab.
#[tauri::command]
pub fn journal_for_work(state: State<'_, AppState>, work_id: String) -> Result<Vec<Entry>> {
    journal::for_entity(&state.conn(), "work", &work_id)
}

/// How many entries are asking to be looked at.
#[tauri::command]
pub fn unread_journal(state: State<'_, AppState>) -> Result<i64> {
    let conn = state.conn();
    journal::unread_count(&conn, &active(&conn)?)
}

/// Mark everything currently unread as seen. Returns how many were.
#[tauri::command]
pub fn mark_journal_read(state: State<'_, AppState>) -> Result<usize> {
    let conn = state.conn();
    journal::mark_read(&conn, &active(&conn)?)
}
