//! A publication's board of ideas for its cover (ADR 0050): what stands on
//! it, the person's own idea, a verdict, and an idea taken into the
//! constructor.

use tauri::State;

use crate::actions;
use crate::cover::idea::{self, CoverBoard, CoverIdea, Verdict};
use crate::error::Result;
use crate::state::AppState;
use crate::trash::Entity;
use crate::work::Work;

/// The board of a publication: its ideas, and the neighbours' covers not yet
/// copied onto it.
#[tauri::command]
pub fn cover_board(state: State<'_, AppState>, id: String) -> Result<CoverBoard> {
    idea::board(&state.conn(), &id)
}

/// Put the person's own idea, in their words, on the board.
#[tauri::command]
pub fn add_own_idea(
    state: State<'_, AppState>,
    work_id: String,
    words: String,
) -> Result<CoverIdea> {
    actions::idea::add_own(&state.conn(), &work_id, &words)
}

/// Star an idea, turn it down, or take the verdict back (`null`).
#[tauri::command]
pub fn judge_idea(
    state: State<'_, AppState>,
    id: String,
    verdict: Option<Verdict>,
) -> Result<CoverIdea> {
    actions::idea::judge(&state.conn(), &id, verdict)
}

/// Judge a neighbour's cover: it is copied onto the board with the verdict.
#[tauri::command]
pub fn judge_sibling_cover(
    state: State<'_, AppState>,
    work_id: String,
    sibling_id: String,
    verdict: Verdict,
) -> Result<CoverIdea> {
    actions::idea::judge_sibling(&state.conn(), &work_id, &sibling_id, verdict)
}

/// Take an idea into its publication's constructor.
#[tauri::command]
pub fn take_idea(state: State<'_, AppState>, id: String) -> Result<Work> {
    actions::idea::take(&state.conn(), &id)
}

/// Take a neighbour's cover into this publication's constructor.
#[tauri::command]
pub fn take_sibling_cover(
    state: State<'_, AppState>,
    work_id: String,
    sibling_id: String,
) -> Result<Work> {
    actions::idea::take_sibling(&state.conn(), &work_id, &sibling_id)
}

/// Take an idea off the board, into the trash.
#[tauri::command]
pub fn delete_idea(state: State<'_, AppState>, id: String) -> Result<String> {
    actions::trash::discard(&state.conn(), Entity::Idea, &id)
}
