//! Finding things: the palette's search, and the catalogue's.

use tauri::State;

use super::active;
use crate::error::Result;
use crate::search::{self, Hit};
use crate::state::AppState;

/// Anything matching a query: works, version bodies, notes, chat messages -
/// one call rather than four, because the palette shows them together and
/// four round trips would arrive out of order.
#[tauri::command]
pub fn search(state: State<'_, AppState>, query: String) -> Result<Vec<Hit>> {
    let conn = state.conn();
    search::find(&conn, &active(&conn)?, &query)
}

/// The works whose text answers a query, best match first: the catalogue
/// wants its list narrowed to the works that say something, not the lines
/// that say it. Ids only — the rows are already on the screen.
#[tauri::command]
pub fn works_matching(state: State<'_, AppState>, query: String) -> Result<Vec<String>> {
    let conn = state.conn();
    search::works_matching(&conn, &active(&conn)?, &query)
}
