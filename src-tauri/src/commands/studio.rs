//! The YouTube Studio pack's commands (v0.159, FR-22 22.1 p.1).

use serde::Serialize;
use tauri::State;

use crate::actions;
use crate::error::{Error, Result};
use crate::release::{Release, ReleasePatch};
use crate::state::AppState;
use crate::studio::{self, StudioPack};

/// The steps of a pack: these works' releases on `door` that are still to
/// go out and have their video.
#[tauri::command(async)]
pub fn studio_pack(
    state: State<'_, AppState>,
    work_ids: Vec<String>,
    door: String,
) -> Result<StudioPack> {
    let today = crate::time::now().get(..10).unwrap_or_default().to_owned();
    studio::pack(&state.conn(), &work_ids, &door, &today)
}

/// What became of a release once its link was given.
#[derive(Debug, Clone, Serialize, ts_rs::TS)]
pub struct StudioDone {
    pub release: Release,
    /// It went out: its day has come. Otherwise it waits on the platform,
    /// booked, its link kept.
    pub released: bool,
}

/// Keep the link the platform gave back for a release: marked out when its
/// day has come, kept on the release while it waits for its day otherwise.
/// A link that is no video's is refused.
#[tauri::command(async)]
pub fn studio_link(
    state: State<'_, AppState>,
    release_id: String,
    url: String,
) -> Result<StudioDone> {
    let url = url.trim().to_owned();
    if !studio::is_video_link(&url) {
        return Err(Error::refused("studio.notAVideoLink").param("url", url));
    }
    let conn = state.conn();
    let found = crate::release::get(&conn, &release_id)?
        .ok_or_else(|| Error::not_found("release", release_id.clone()))?;
    let today = crate::time::now().get(..10).unwrap_or_default().to_owned();
    let day = found
        .scheduled_at
        .as_deref()
        .and_then(|at| at.get(..10))
        .map(str::to_owned);
    if day.as_deref().is_none_or(|day| day <= today.as_str()) {
        let release = actions::release::mark_released(&conn, &release_id, Some(url), day, None)?;
        return Ok(StudioDone {
            release,
            released: true,
        });
    }
    let release = actions::release::update(
        &conn,
        &release_id,
        ReleasePatch {
            url: Some(Some(url)),
            ..ReleasePatch::default()
        },
    )?;
    Ok(StudioDone {
        release,
        released: false,
    })
}
