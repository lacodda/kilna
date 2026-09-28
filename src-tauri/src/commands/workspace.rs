//! The workspace itself: its profiles, and where it lives on disk.

use tauri::State;

use crate::actions;
use crate::error::Result;
use crate::profile::{self, Profile, Workspace};
use crate::state::AppState;

/// Everything the status screen needs in one round trip.
#[tauri::command]
pub fn get_workspace(state: State<'_, AppState>) -> Result<Workspace> {
    profile::workspace(&state.conn())
}

#[tauri::command]
pub fn list_profiles(state: State<'_, AppState>) -> Result<Vec<Profile>> {
    profile::list(&state.conn())
}

#[tauri::command]
pub fn activate_profile(state: State<'_, AppState>, id: String) -> Result<()> {
    profile::activate(&state.conn(), &id)
}

#[tauri::command]
pub fn update_profile_config(
    state: State<'_, AppState>,
    id: String,
    config: profile::config::ProfileConfig,
) -> Result<Profile> {
    actions::profile::update_config(&state.conn(), &id, &config)
}

/// Where the workspace file lives, so the person can find or replace it.
#[tauri::command]
pub fn workspace_path(state: State<'_, AppState>) -> String {
    state.path().display().to_string()
}

/// Where the application's log is being written, when it is: what to attach
/// to a report of "something odd happened yesterday".
#[tauri::command]
pub fn log_path() -> Option<String> {
    crate::log::path().map(|path| path.display().to_string())
}

/// Write a failure the window caught into the application's log.
///
/// A screen that stopped working used to leave its detail only in the
/// window's console, which a release build does not show; the error boundary
/// sends it here so it lands beside everything else that went wrong.
#[tauri::command]
pub fn log_window_error(message: String) {
    crate::log::error("window", &message);
}
