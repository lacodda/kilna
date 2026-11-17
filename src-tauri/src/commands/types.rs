//! Types of space (v0.112, ADR 0079), and the profile as a document: kept as
//! what a new space is made of, written out to a file, read back from one.
//!
//! A type lives in the application's database - this machine's record, off a
//! space's operations log as the list of spaces is. Reading a document into
//! the open space's profile is not done here at all: the window takes the
//! document as the editor's draft, and it is saved by the gesture every edit
//! of the profile is (`profile.update`), or thrown away.

use tauri::State;

use crate::error::{Error, Result};
use crate::profile::config::ProfileConfig;
use crate::space::types::{self, ProfileDocument, SpaceType};
use crate::state::AppState;

/// The open space's profile as a document, whole.
fn open_profile(state: &AppState) -> Result<ProfileDocument> {
    let profile = crate::profile::active(&state.conn())?
        .ok_or_else(|| Error::refused("profile.noneActive"))?;
    Ok(ProfileDocument {
        key: profile.key,
        name: profile.name,
        description: profile.description,
        config: profile.config,
    })
}

/// Every type this machine keeps.
#[tauri::command]
pub fn list_space_types(state: State<'_, AppState>) -> Result<Vec<SpaceType>> {
    types::list(&state.app())
}

/// Keep the open space's profile as a type, under a name: what is stored,
/// not what the editor holds unsaved.
#[tauri::command]
pub fn save_space_type(state: State<'_, AppState>, name: String) -> Result<SpaceType> {
    let mut document = open_profile(&state)?;
    document.name = name;
    types::add(&state.app(), document)
}

/// Keep a document read from a file as a type.
#[tauri::command]
pub fn add_space_type(state: State<'_, AppState>, document: ProfileDocument) -> Result<SpaceType> {
    types::add(&state.app(), document)
}

#[tauri::command]
pub fn rename_space_type(
    state: State<'_, AppState>,
    id: String,
    name: String,
) -> Result<SpaceType> {
    types::rename(&state.app(), &id, &name)
}

#[tauri::command]
pub fn delete_space_type(state: State<'_, AppState>, id: String) -> Result<()> {
    types::delete(&state.app(), &id)
}

/// A type's document as the text of a file, for the window to write where
/// the person says (`write_text_file`).
#[tauri::command]
pub fn space_type_document(state: State<'_, AppState>, id: String) -> Result<String> {
    types::document(&state.app(), &id)?.to_text()
}

/// The open space's profile as the text of a file.
#[tauri::command]
pub fn profile_document(state: State<'_, AppState>) -> Result<String> {
    open_profile(&state)?.to_text()
}

/// Read a profile document from a file the person picked: checked, and
/// refused with every problem named when anything in it is wrong. The window
/// has no right to read files, and is not given one for this.
#[tauri::command(async)]
pub fn read_profile_document(path: String) -> Result<ProfileDocument> {
    types::read_document(std::path::Path::new(&path))
}

/// What is wrong with a profile as the editor holds it, each problem a
/// reason with its place: asked as the person edits, so a word that will not
/// save says so beside the field rather than at the Save button.
#[tauri::command]
pub fn check_profile_config(config: ProfileConfig) -> Vec<crate::error::Reason> {
    config.problems()
}
