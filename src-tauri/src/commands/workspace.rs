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
pub fn update_profile_config(
    state: State<'_, AppState>,
    id: String,
    config: profile::config::ProfileConfig,
) -> Result<Profile> {
    actions::profile::update_config(&state.conn(), &id, &config)
}

/// The versions kilna ships waiting beside the owner's entries of a profile
/// (ADR 0087): what Settings offers to be kept or taken.
#[tauri::command]
pub fn shipped_offers(
    state: State<'_, AppState>,
    id: String,
) -> Result<Vec<profile::shipped::ShippedOffer>> {
    profile::shipped::offers(&state.conn(), &id)
}

/// Where each shipped entry of one list stands in a profile, with the
/// version a reset puts back.
#[tauri::command]
pub fn shipped_standing(
    state: State<'_, AppState>,
    id: String,
    collection: String,
) -> Result<Vec<profile::shipped::EntryStanding>> {
    profile::shipped::standing(&state.conn(), &id, &collection)
}

/// A skill as the assistant would be handed it, for a kind of work: read
/// from the action as it stands in the editor, saved or not.
#[tauri::command]
pub fn preview_skill(
    state: State<'_, AppState>,
    kind: String,
    action: crate::assistant::prompt::PromptTemplate,
) -> Result<crate::assistant::task::SkillPreview> {
    let profile = profile::active(&state.conn())?
        .ok_or_else(|| crate::error::Error::refused("profile.noneActive"))?;
    Ok(crate::assistant::task::preview(
        &profile.config,
        &kind,
        &action,
    ))
}

/// The skills a file holds, to be laid over the profile's draft.
#[tauri::command]
pub fn read_skill_set(path: String) -> Result<Vec<crate::assistant::prompt::PromptTemplate>> {
    crate::assistant::prompt::read_set(std::path::Path::new(&path))
}

/// Take the shipped version of one entry over the owner's.
#[tauri::command]
pub fn take_shipped(
    state: State<'_, AppState>,
    id: String,
    offer: profile::shipped::OfferRef,
) -> Result<Profile> {
    profile::shipped::take(&state.conn(), &id, &offer)
}

/// Keep the owner's entries over the shipped versions waiting beside them.
#[tauri::command]
pub fn keep_mine(
    state: State<'_, AppState>,
    id: String,
    offers: Vec<profile::shipped::OfferRef>,
) -> Result<()> {
    profile::shipped::keep(&state.conn(), &id, &offers)
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
