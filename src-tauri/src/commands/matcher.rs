//! A batch of files sorted into their works (v0.157, FR-21 21.2 p.2): matched
//! without guessing, shown in a table, taken in only as confirmed, and the
//! duplicates let go into the recycle bin when the person says so.

use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::{LazyLock, Mutex};

use serde::{Deserialize, Serialize};
use tauri::State;

use super::active;
use crate::actions::media::{self, MediaRole};
use crate::error::{Error, Result};
use crate::matcher::{self, MediaMatch};
use crate::state::AppState;

/// The files a batch was handed: the only ones a preview may read or the
/// recycle bin may take - a command that reads any path asked of it would
/// read anything.
static HANDED: LazyLock<Mutex<HashSet<PathBuf>>> = LazyLock::new(|| Mutex::new(HashSet::new()));

fn handed(path: &str) -> bool {
    HANDED
        .lock()
        .is_ok_and(|known| known.contains(&PathBuf::from(path)))
}

/// Match a batch of files against a collection's works, or every work of
/// the space. Files only - a folder in the batch is opened one level down.
#[tauri::command(async)]
pub fn match_media(
    state: State<'_, AppState>,
    paths: Vec<String>,
    collection_id: Option<String>,
) -> Result<Vec<MediaMatch>> {
    let mut files = Vec::new();
    for path in paths {
        let at = PathBuf::from(&path);
        if at.is_dir() {
            let mut inside: Vec<String> = std::fs::read_dir(&at)?
                .flatten()
                .map(|entry| entry.path())
                .filter(|path| path.is_file())
                .map(|path| path.to_string_lossy().into_owned())
                .collect();
            inside.sort();
            files.extend(inside);
        } else if at.is_file() {
            files.push(path);
        }
    }
    if let Ok(mut known) = HANDED.lock() {
        known.extend(files.iter().map(PathBuf::from));
    }
    let conn = state.conn();
    let works = matcher::candidates(&conn, &active(&conn)?, collection_id.as_deref())?;
    Ok(matcher::match_files(&files, &works))
}

/// A picture of the batch, for its row in the table.
#[tauri::command(async)]
pub fn media_preview(path: String) -> Result<tauri::ipc::Response> {
    if !handed(&path) {
        return Err(Error::refused("media.notHanded").param("path", path));
    }
    Ok(tauri::ipc::Response::new(std::fs::read(&path)?))
}

/// One file of a batch as the person confirmed it.
#[derive(Debug, Clone, Deserialize, ts_rs::TS)]
pub struct MediaPick {
    pub path: String,
    pub work_id: String,
    pub role: MediaRole,
}

/// How a batch went.
#[derive(Debug, Clone, Default, Serialize, ts_rs::TS)]
pub struct MediaBatchReport {
    pub taken: u32,
    pub trashed: u32,
    /// The files that could not be taken, each with why.
    pub failed: Vec<MediaBatchFailure>,
}

/// A file of a batch that could not be taken in.
#[derive(Debug, Clone, Serialize, ts_rs::TS)]
pub struct MediaBatchFailure {
    pub name: String,
    pub reason: String,
}

/// Take the confirmed files into their works, each under a name of its
/// work's and as its role, and put the ones let go into the recycle bin.
/// One file that fails is said and the rest go on.
#[tauri::command(async)]
pub fn take_media_batch(
    state: State<'_, AppState>,
    picks: Vec<MediaPick>,
    trash: Vec<String>,
) -> Result<MediaBatchReport> {
    let media_dir = state.media_dir()?;
    let mut report = MediaBatchReport::default();
    for pick in picks {
        let name = std::path::Path::new(&pick.path)
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| pick.path.clone());
        let taken = media::take(
            &state.conn(),
            &media_dir,
            &pick.work_id,
            std::path::Path::new(&pick.path),
            pick.role,
            true,
        );
        match taken {
            Ok(_) => report.taken += 1,
            Err(cause) => report.failed.push(MediaBatchFailure {
                name,
                reason: cause.to_string(),
            }),
        }
    }
    for path in trash {
        if !handed(&path) {
            continue;
        }
        match ::trash::delete(&path) {
            Ok(()) => report.trashed += 1,
            Err(cause) => report.failed.push(MediaBatchFailure {
                name: path,
                reason: cause.to_string(),
            }),
        }
    }
    Ok(report)
}
