//! Scores: a work judged along the craft's axes, and what each kind of release
//! makes of it.

use tauri::State;

use crate::actions;
use crate::error::{Error, Result};
use crate::profile;
use crate::score::{self, NewScore, Score};
use crate::state::AppState;
use crate::trash::Entity;
use crate::work;

#[tauri::command]
pub fn score_work(state: State<'_, AppState>, work_id: String, score: NewScore) -> Result<Score> {
    actions::score::create(&state.conn(), &work_id, score)
}

#[tauri::command]
pub fn score_history(state: State<'_, AppState>, work_id: String) -> Result<Vec<Score>> {
    score::history(&state.conn(), &work_id)
}

#[tauri::command]
pub fn latest_score(state: State<'_, AppState>, work_id: String) -> Result<Option<Score>> {
    score::latest(&state.conn(), &work_id)
}

/// What each release kind makes of this work's latest answers.
///
/// Computed rather than stored: the weights live in the profile, so a verdict
/// saved beside the score would be stale the moment the craft changed its
/// mind. An unscored work has no verdicts, which is not the same as a work
/// every kind rates zero.
#[tauri::command]
pub fn kind_verdicts(
    state: State<'_, AppState>,
    work_id: String,
) -> Result<Vec<profile::config::KindVerdict>> {
    let conn = state.conn();
    let Some(latest) = score::latest(&conn, &work_id)? else {
        return Ok(Vec::new());
    };
    let profile = profile::active(&conn)?.ok_or_else(|| Error::refused("profile.noneActive"))?;
    let work = work::get(&conn, &work_id)?.ok_or_else(|| Error::not_found("work", &work_id))?;
    Ok(profile.config.verdicts(&work.kind, &latest.axes))
}

#[tauri::command]
pub fn delete_score(state: State<'_, AppState>, id: String) -> Result<String> {
    actions::trash::discard(&state.conn(), Entity::Score, &id)
}
