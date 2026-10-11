//! The calendar's closed days: a holiday, a fortnight away.

use tauri::State;

use super::active;
use crate::actions;
use crate::error::Result;
use crate::state::AppState;
use crate::time_off::{self, NewTimeOff, TimeOff, TimeOffPatch};

/// Every span of closed days in the active profile, earliest first.
#[tauri::command]
pub fn list_time_off(state: State<'_, AppState>) -> Result<Vec<TimeOff>> {
    let conn = state.conn();
    time_off::list(&conn, &active(&conn)?)
}

#[tauri::command]
pub fn create_time_off(state: State<'_, AppState>, time_off: NewTimeOff) -> Result<TimeOff> {
    actions::time_off::create(&state.conn(), time_off)
}

#[tauri::command]
pub fn update_time_off(
    state: State<'_, AppState>,
    id: String,
    patch: TimeOffPatch,
) -> Result<TimeOff> {
    actions::time_off::update(&state.conn(), &id, patch)
}

#[tauri::command]
pub fn delete_time_off(state: State<'_, AppState>, id: String) -> Result<()> {
    actions::time_off::delete(&state.conn(), &id)
}
