//! The boards of a work's conception (ADR 0065): what stands on them, the
//! person's own card, a verdict, a card written or moved, a topic renamed -
//! and the order a writer works from.

use tauri::State;

use crate::actions;
use crate::board::{self, BoardCard, BoardView, CardPatch, Verdict};
use crate::error::{Error, Result};
use crate::state::AppState;
use crate::trash::Entity;

/// Every board a work lays out, with the cards on each.
#[tauri::command]
pub fn work_boards(state: State<'_, AppState>, id: String) -> Result<Vec<BoardView>> {
    let conn = state.conn();
    let work = crate::work::get(&conn, &id)?.ok_or_else(|| Error::not_found("work", &id))?;
    board::boards(&conn, &work)
}

/// Put the person's own card, in their words, on a board: the words are its
/// headline.
#[tauri::command]
pub fn add_own_card(
    state: State<'_, AppState>,
    work_id: String,
    board: String,
    kind: Option<String>,
    words: String,
) -> Result<BoardCard> {
    actions::board::add_own(&state.conn(), &work_id, &board, kind.as_deref(), &words)
}

/// Change what a card says, or its kind.
#[tauri::command]
pub fn update_card(state: State<'_, AppState>, id: String, patch: CardPatch) -> Result<BoardCard> {
    actions::board::update(&state.conn(), &id, patch)
}

/// Star a card, turn it down, or take the verdict back (`null`).
#[tauri::command]
pub fn judge_card(
    state: State<'_, AppState>,
    id: String,
    verdict: Option<Verdict>,
) -> Result<BoardCard> {
    actions::board::judge(&state.conn(), &id, verdict)
}

/// Move a card under a topic, to a place among that topic's cards.
#[tauri::command]
pub fn move_card(
    state: State<'_, AppState>,
    id: String,
    topic: String,
    index: usize,
) -> Result<()> {
    actions::board::move_card(&state.conn(), &id, &topic, index)
}

/// Rename a topic of a board, or merge it into another by its name.
#[tauri::command]
pub fn retopic_cards(
    state: State<'_, AppState>,
    work_id: String,
    board: String,
    from: String,
    to: String,
) -> Result<()> {
    actions::board::retopic(&state.conn(), &work_id, &board, &from, &to)
}

/// Take a card off its board, into the trash.
#[tauri::command]
pub fn delete_card(state: State<'_, AppState>, id: String) -> Result<String> {
    actions::trash::discard(&state.conn(), Entity::Card, &id)
}

/// Keep a chip of a card - an image, a name, a place - as a phrase of the
/// work the card is on (ADR 0066).
#[tauri::command]
pub fn chip_to_phrase(
    state: State<'_, AppState>,
    card_id: String,
    chip: String,
) -> Result<crate::note::Note> {
    actions::board::chip_to_phrase(&state.conn(), &card_id, &chip)
}

/// The works an entry of the library has worked in: what the entry shows in
/// the library.
#[tauri::command]
pub fn entry_worked_in(
    state: State<'_, AppState>,
    id: String,
) -> Result<Vec<crate::library::WorkedIn>> {
    crate::library::worked_in(&state.conn(), &id)
}

/// The whole order a writer works from, in markdown: what "Copy the order"
/// copies, and what an agent reads with `brief`.
#[tauri::command]
pub fn conception_order(state: State<'_, AppState>, id: String) -> Result<String> {
    let conn = state.conn();
    let work = crate::work::get(&conn, &id)?.ok_or_else(|| Error::not_found("work", &id))?;
    crate::conception::order(&conn, &work)
}
