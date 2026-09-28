//! The workspace's data going out and coming in: backups, exports, imports,
//! and a text the window composed written to a file.
//!
//! Off the main thread, every one of them: their work is files, and a
//! synchronous command runs on the thread that draws the window - a backup
//! copying a folder of clips froze it for as long as the copy took.

use tauri::State;

use crate::error::{Error, Result};
use crate::exchange::backup;
use crate::exchange::export::{self, ExportReport};
use crate::exchange::import::{self, ImportReport};
use crate::exchange::package;
use crate::state::AppState;

/// Write a text the window composed to a place the person picked.
///
/// The window has no filesystem permissions and is not given any for this:
/// it already holds the text, and what it lacks is the right to write a file.
/// The path is the one the save dialog returned, so the person chose it; this
/// refuses only to write a directory, which the dialog cannot return but a
/// caller could pass.
#[tauri::command(async)]
pub fn write_text_file(path: String, text: String) -> Result<String> {
    let target = std::path::Path::new(&path);
    if target.is_dir() {
        return Err(Error::refused("file.isDirectory").param("path", path.as_str()));
    }
    std::fs::write(target, text).map_err(|cause| {
        Error::refused("file.notWritten")
            .param("path", path.as_str())
            .param("cause", cause.to_string())
    })?;
    Ok(path)
}

/// Write the active profile out as markdown.
#[tauri::command(async)]
pub fn export_markdown(state: State<'_, AppState>, directory: String) -> Result<ExportReport> {
    export::to_markdown(&state.conn(), std::path::Path::new(&directory))
}

/// Pack a work into a folder: its board with every prompt, the pictures under
/// names that say what they are, and what its releases go out as. Reads only.
#[tauri::command(async)]
pub fn export_package(
    state: State<'_, AppState>,
    work_id: String,
    directory: String,
) -> Result<package::PackageReport> {
    package::write(&state.conn(), &work_id, std::path::Path::new(&directory))
}

/// Whether a work has anything worth packing: a board, or something written
/// about a release.
#[tauri::command]
pub fn can_export_package(state: State<'_, AppState>, work_id: String) -> Result<bool> {
    package::has_anything(&state.conn(), &work_id)
}

/// Copy the workspace somewhere safe — the database and the files with it.
///
/// The database is held only while SQLite copies it; the pictures are copied
/// with the lock let go, so the rest of the app keeps working meanwhile.
#[tauri::command(async)]
pub fn backup_workspace(state: State<'_, AppState>, destination: String) -> Result<String> {
    // Asked for before the connection is taken, because preparing it may
    // create the directory and that is not work to do under the lock.
    let media = state.media_dir().ok();
    let destination = std::path::Path::new(&destination);
    let written = backup::write_database(&state.conn(), destination)?;
    backup::copy_media(destination, media.as_deref())?;
    Ok(written.display().to_string())
}

/// Suggested file name for a backup taken now.
#[tauri::command]
pub fn suggested_backup_name() -> String {
    backup::suggested_name(&crate::time::now())
}

/// Bring in a slice of a predecessor workspace. Existing titles are skipped.
#[tauri::command(async)]
pub fn import_legacy(state: State<'_, AppState>, source: String) -> Result<ImportReport> {
    let conn = state.conn();
    let profile_id = super::active(&conn)?;
    import::from_legacy(&conn, std::path::Path::new(&source), &profile_id)
}
