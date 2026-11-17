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

/// Open a work's folder in the system's file manager, making it first if it
/// is not there (FR-04): a folder is made when it is first needed, and
/// opening it is needing it. There is no "make the folder" of its own - the
/// step stood between a person and the place they wanted to drop a file in.
#[tauri::command(async)]
pub fn open_work_folder(state: State<'_, AppState>, work_id: String) -> Result<WorkFolder> {
    let folder = {
        let conn = state.conn();
        let (work, config) = work_and_config(&conn, &work_id)?;
        folder::ensure(&conn, &config, &work)?
    };
    if let Some(path) = folder.path.as_deref() {
        tauri_plugin_opener::open_path(Path::new(path), None::<&str>).map_err(|cause| {
            Error::refused("folder.openFailed")
                .param("path", path)
                .param("cause", cause.to_string())
        })?;
    }
    Ok(folder)
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

/// The folders the window may read a file from: every media folder, and the
/// workspace's own files.
fn places(state: &State<'_, AppState>) -> Result<Vec<std::path::PathBuf>> {
    let mut places: Vec<std::path::PathBuf> = folder::roots(&state.conn())?
        .into_iter()
        .map(std::path::PathBuf::from)
        .collect();
    places.push(state.media_dir()?);
    Ok(places)
}

/// A text, read for the viewer: a lyric, a subtitle track, notes (FR-14).
/// Only under the media folder or the workspace's own files, only a text,
/// and no more than the viewer shows.
#[tauri::command(async)]
pub fn read_text_file(state: State<'_, AppState>, path: String) -> Result<folder::TextFile> {
    folder::read_text(Path::new(&path), &places(&state)?)
}

/// Save a copy of a file kilna looks at where the person said.
#[tauri::command(async)]
pub fn save_file_copy(
    state: State<'_, AppState>,
    source: String,
    destination: String,
) -> Result<()> {
    folder::save_copy(
        Path::new(&source),
        &places(&state)?,
        Path::new(&destination),
    )
}

/// The folder of a work that is there, or why it is not.
fn folder_there(
    conn: &rusqlite::Connection,
    work: &crate::work::Work,
    config: &crate::profile::config::ProfileConfig,
) -> Result<std::path::PathBuf> {
    let found = folder::locate(conn, config, work)?;
    match (found.state, found.path) {
        (folder::FolderState::Found, Some(path)) => Ok(std::path::PathBuf::from(path)),
        _ => Err(Error::refused("folder.fileMissing").param("path", work.title.clone())),
    }
}

/// Rename a file of a work's folder (FR-14). When it is the file the work
/// was said to be finished as, the work's choice follows the name - through
/// the ordinary edit of a work, so an undo puts the choice back.
#[tauri::command(async)]
pub fn rename_folder_file(
    state: State<'_, AppState>,
    work_id: String,
    relative: String,
    name: String,
) -> Result<WorkFolder> {
    let conn = state.conn();
    let (work, config) = work_and_config(&conn, &work_id)?;
    let at = folder_there(&conn, &work, &config)?;
    let renamed = folder::rename_file(&at, &relative, &name)?;
    if work.file.as_deref() == Some(folder::inside(&relative)?.as_str()) && renamed != relative {
        crate::actions::work::update(
            &conn,
            &work_id,
            crate::work::WorkPatch {
                file: Some(Some(renamed)),
                ..Default::default()
            },
        )?;
    }
    let work =
        crate::work::get(&conn, &work_id)?.ok_or_else(|| Error::not_found("work", &work_id))?;
    folder::of_work(&conn, &config, &work)
}

/// Put a file of a work's folder into the system's recycle bin (FR-14) -
/// never deleted outright: what a person throws away by a slip of the hand
/// is brought back where every other file they threw away is. The folder
/// itself is never removed this way.
#[tauri::command(async)]
pub fn trash_folder_file(
    state: State<'_, AppState>,
    work_id: String,
    relative: String,
) -> Result<WorkFolder> {
    let conn = state.conn();
    let (work, config) = work_and_config(&conn, &work_id)?;
    let at = folder_there(&conn, &work, &config)?;
    let path = folder::file_in(&at, &relative)?;
    trash::delete(&path).map_err(|cause| {
        Error::refused("folder.trashFailed")
            .param("path", relative.clone())
            .param("cause", cause.to_string())
    })?;
    folder::of_work(&conn, &config, &work)
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
