//! Looking after the open space (v0.114): the copies kilna keeps of it and
//! putting one back, how much room it takes and giving some back, an
//! examination of it, and how long its history is kept.
//!
//! Off the main thread where the work is files or the whole database, as in
//! `data.rs`.

use tauri::State;

use crate::error::{Error, Result};
use crate::exchange::backup::{self, KeptBackup};
use crate::space;
use crate::state::AppState;
use crate::upkeep::{self, Compacted, Examination, Storage};

/// The copies kilna keeps of the open space, newest first: one a day, and
/// one before each upgrade and each restore.
#[tauri::command]
pub fn kept_backups(state: State<'_, AppState>) -> Result<Vec<KeptBackup>> {
    backup::kept(state.folder().path())
}

/// Put a backup back over the open space - one kilna kept, or a file the
/// person took - without a restart. What stood before is kept first, and the
/// path of that copy is what comes back.
///
/// Refused while anything is going, as opening another space is: a run would
/// finish into a workspace that is no longer the one it began in.
#[tauri::command(async)]
pub fn restore_backup(state: State<'_, AppState>, path: String) -> Result<String> {
    super::spaces::nothing_going(&state)?;
    let aside = state.restore(std::path::Path::new(&path))?;
    crate::log::info(
        "backup",
        &format!("restored {path}; what stood before is {}", aside.display()),
    );
    Ok(aside.display().to_string())
}

/// How much room the open space takes, by what takes it.
#[tauri::command(async)]
pub fn storage_report(state: State<'_, AppState>) -> Result<Storage> {
    let folder = state.folder();
    let media = state.media_dir()?;
    upkeep::storage(&state.conn(), folder.path(), &folder.database(), &media)
}

/// Rewrite the open space's database without the room it no longer uses.
/// No row changes.
#[tauri::command(async)]
pub fn compact_workspace(state: State<'_, AppState>) -> Result<Compacted> {
    if state.held_by().is_some() {
        return Err(Error::refused("space.readOnly"));
    }
    let database = state.path();
    upkeep::compact(&state.conn(), &database)
}

/// Examine the open space: the file itself, rows that name what is not
/// there, files lost from under their rows and files nothing names. Reads
/// only.
#[tauri::command(async)]
pub fn examine_workspace(state: State<'_, AppState>) -> Result<Examination> {
    let media = state.media_dir()?;
    upkeep::examine(&state.conn(), &media)
}

/// How long the open space keeps the read lines of its history, and where
/// they go after.
#[derive(Debug, Clone, serde::Serialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
pub struct HistoryKeeping {
    /// Days; `0` is for good.
    pub days: u32,
    /// The folder of pages the lines go to, a month each.
    pub archive: String,
}

fn keeping(state: &AppState) -> Result<HistoryKeeping> {
    let folder = state.folder();
    let manifest = space::Manifest::read(folder.path())?;
    Ok(HistoryKeeping {
        days: manifest
            .history_days
            .unwrap_or(crate::journal::RETENTION_DAYS),
        archive: folder.path().join(space::HISTORY_DIR).display().to_string(),
    })
}

/// How long the open space keeps its history.
#[tauri::command]
pub fn history_keeping(state: State<'_, AppState>) -> Result<HistoryKeeping> {
    keeping(&state)
}

/// Say how long the open space keeps its history: a number of days, or `0`
/// for good. Written in the space's own file, so it travels with the folder.
#[tauri::command]
pub fn set_history_days(state: State<'_, AppState>, days: u32) -> Result<HistoryKeeping> {
    space::keep_history(&state.folder(), days)?;
    keeping(&state)
}
