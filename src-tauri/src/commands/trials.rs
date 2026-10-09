//! An experiment's board of trials (ADR 0061): what stands on it, a trial
//! written, heard, judged and varied, and what a kept one becomes.

use serde::Serialize;
use tauri::State;

use crate::actions;
use crate::error::Result;
use crate::lab::{NewTrial, Trial, TrialBoard, TrialPatch, Verdict};
use crate::state::AppState;
use crate::style_brick::{NewStyleBrick, StyleBrick};
use crate::trash::Entity;
use crate::work::Work;
use crate::work::version::Version;

/// The board of an experiment, every trial with what it became.
#[tauri::command]
pub fn trial_board(state: State<'_, AppState>, id: String) -> Result<TrialBoard> {
    crate::lab::trial::board(&state.conn(), &id)
}

/// Put a trial on a board: empty, written from the dictionary, or anything
/// the window composed.
#[tauri::command]
pub fn create_trial(state: State<'_, AppState>, trial: NewTrial) -> Result<Trial> {
    actions::trial::create(&state.conn(), trial)
}

/// Change a trial.
#[tauri::command]
pub fn update_trial(state: State<'_, AppState>, id: String, patch: TrialPatch) -> Result<Trial> {
    actions::trial::update(&state.conn(), &id, patch)
}

/// Keep a trial, drop it, or take the verdict back (`null`).
#[tauri::command]
pub fn judge_trial(
    state: State<'_, AppState>,
    id: String,
    verdict: Option<Verdict>,
) -> Result<Trial> {
    actions::trial::judge(&state.conn(), &id, verdict)
}

/// A variation of a trial, with what is to move.
#[tauri::command]
pub fn vary_trial(
    state: State<'_, AppState>,
    id: String,
    angle: String,
    series: Option<String>,
) -> Result<Trial> {
    actions::trial::vary(&state.conn(), &id, &angle, series.as_deref())
}

/// A trial from a work's text in the role the lab keeps trials in: a song's
/// style, to be reworked.
#[tauri::command]
pub fn trial_from_version(
    state: State<'_, AppState>,
    work_id: String,
    version_id: String,
    series: String,
) -> Result<Trial> {
    actions::trial::from_version(&state.conn(), &work_id, &version_id, &series)
}

/// Take a trial off the board, into the trash.
#[tauri::command]
pub fn delete_trial(state: State<'_, AppState>, id: String) -> Result<String> {
    actions::trash::discard(&state.conn(), Entity::Trial, &id)
}

/// Take a kept trial into a work, as a version beside its current one -
/// current only when asked.
#[tauri::command]
pub fn harvest_trial(
    state: State<'_, AppState>,
    id: String,
    work_id: String,
    label: Option<String>,
    make_current: Option<bool>,
) -> Result<Version> {
    actions::trial::harvest_into(
        &state.conn(),
        &id,
        &work_id,
        label.as_deref(),
        make_current.unwrap_or(false),
    )
}

/// Cut a phrase of the dictionary out of a kept trial.
#[tauri::command]
pub fn harvest_trial_phrase(
    state: State<'_, AppState>,
    id: String,
    brick: NewStyleBrick,
) -> Result<StyleBrick> {
    actions::trial::harvest_phrase(&state.conn(), &id, brick)
}

/// Make a new work of a kept trial.
#[tauri::command]
pub fn harvest_trial_work(
    state: State<'_, AppState>,
    id: String,
    kind: String,
    title: String,
) -> Result<Work> {
    actions::trial::harvest_work(&state.conn(), &id, &kind, &title)
}

/// An experiment made from a work, and the trial its text became.
#[derive(Debug, Clone, Serialize, ts_rs::TS)]
pub struct MadeExperiment {
    pub work: Work,
    /// The work's text as the first trial, when it had one.
    pub core: Option<Trial>,
}

/// Make an experiment that reworks a work: its newest text in the role the
/// lab keeps trials in is the first trial, in `series`.
#[tauri::command]
pub fn make_experiment(
    state: State<'_, AppState>,
    source_id: String,
    title: String,
    series: String,
) -> Result<MadeExperiment> {
    let (work, core) = actions::trial::experiment_from(&state.conn(), &source_id, &title, &series)?;
    Ok(MadeExperiment { work, core })
}
