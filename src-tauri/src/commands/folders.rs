//! Folders on disk the workspace looks at: the media folder, and each work's
//! folder under it (ADR 0057).

use std::path::Path;

use tauri::{AppHandle, Manager, State};

use super::active;
use crate::error::{Error, Result};
use crate::folder::{self, WorkFolder};
use crate::state::AppState;

/// The media folder of the active workspace on this machine, if one is set.
#[tauri::command]
pub fn media_root(state: State<'_, AppState>) -> Result<Option<String>> {
    let conn = state.conn();
    folder::root(&conn, &active(&conn)?)
}

/// Set the media folder of the active workspace, or forget it.
///
/// Written past the actions and the log, as which profile is open is: a
/// place on this machine is not part of the workspace a replay rebuilds or
/// a sync carries (ADR 0057), and on the next device it would name nothing.
///
/// The window may read what is under it from now on - the scope is widened
/// here as it is at start, because waiting for a restart to see the files
/// is a setting that looks broken. A folder set before stays readable until
/// the window closes: narrowing the scope could shut out the new folder when
/// it lies inside the old one, and the old one was the person's own choice.
#[tauri::command]
pub fn set_media_root(
    app: AppHandle,
    state: State<'_, AppState>,
    path: Option<String>,
) -> Result<Option<String>> {
    let conn = state.conn();
    let profile_id = active(&conn)?;
    folder::set_root(&conn, &profile_id, path.as_deref(), &crate::time::now())?;
    let root = folder::root(&conn, &profile_id)?;
    if let Some(root) = &root {
        allow(&app, root);
    }
    Ok(root)
}

/// A work's folder on disk, and the files in it.
#[tauri::command(async)]
pub fn work_folder(state: State<'_, AppState>, work_id: String) -> Result<WorkFolder> {
    let conn = state.conn();
    let (work, config) = work_and_config(&conn, &work_id)?;
    folder::of_work(&conn, &config, &work)
}

/// Make a work's folder under the media folder, so it is there to be filled.
#[tauri::command(async)]
pub fn create_work_folder(state: State<'_, AppState>, work_id: String) -> Result<WorkFolder> {
    let conn = state.conn();
    let (work, config) = work_and_config(&conn, &work_id)?;
    folder::create(&conn, &config, &work)
}

/// Where the workspace keeps its own files - what a note's `media/<name>`
/// is read against.
#[tauri::command]
pub fn media_directory(state: State<'_, AppState>) -> Result<String> {
    Ok(state.media_dir()?.to_string_lossy().into_owned())
}

/// Open a file kilna looks at in the program the system opens it with - a
/// take in the player, a still in the viewer. Only a picture, a clip, a
/// sound or a plain document, and only under the media folder or the
/// workspace's own files: the window names the path, and a path the window
/// names must not become a way to run a program.
#[tauri::command(async)]
pub fn open_media(state: State<'_, AppState>, path: String) -> Result<()> {
    let conn = state.conn();
    let mut places: Vec<std::path::PathBuf> = folder::roots(&conn)?
        .into_iter()
        .map(std::path::PathBuf::from)
        .collect();
    places.push(state.media_dir()?);
    let target = Path::new(&path);
    if !folder::may_open(target, &places) {
        return Err(Error::refused("folder.notOpenable").param("path", path));
    }
    tauri_plugin_opener::open_path(target, None::<&str>).map_err(|cause| {
        Error::refused("folder.openFailed")
            .param("path", path.clone())
            .param("cause", cause.to_string())
    })
}

fn work_and_config(
    conn: &rusqlite::Connection,
    work_id: &str,
) -> Result<(crate::work::Work, crate::profile::config::ProfileConfig)> {
    let work = crate::work::get(conn, work_id)?.ok_or_else(|| Error::not_found("work", work_id))?;
    let config = crate::profile::config_for(conn, &work.profile_id)?;
    Ok((work, config))
}

/// Let the window read the files under `root`. Said, not raised, when it
/// cannot: the setting still holds, and the Files tab then shows names where
/// pictures would be.
pub fn allow(app: &AppHandle, root: &str) {
    if let Err(cause) = app
        .asset_protocol_scope()
        .allow_directory(Path::new(root), true)
    {
        crate::log::error(
            "folder",
            &format!("the window may not read {root}: {cause}"),
        );
    }
}
