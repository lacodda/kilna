//! YouTube Studio's export read and kept (v0.160, FR-22 22.1 p.2).

use tauri::State;

use super::active;
use crate::actions;
use crate::analytics::{self, AnalyticsImport};
use crate::error::Result;
use crate::state::AppState;

/// Read an export against the space's releases: what each row is, and the
/// release it belongs to. Nothing is kept.
#[tauri::command(async)]
pub fn read_analytics(state: State<'_, AppState>, path: String) -> Result<AnalyticsImport> {
    let text = crate::plan_import::text_of(&std::fs::read(&path)?);
    let conn = state.conn();
    analytics::read(&conn, &active(&conn)?, &text)
}

/// Read an export again and keep the numbers of every row matched to a
/// release. How many releases were given numbers.
#[tauri::command(async)]
pub fn keep_analytics(state: State<'_, AppState>, path: String) -> Result<u32> {
    let text = crate::plan_import::text_of(&std::fs::read(&path)?);
    let conn = state.conn();
    let read = analytics::read(&conn, &active(&conn)?, &text)?;
    actions::analytics::keep(&conn, &read)
}
