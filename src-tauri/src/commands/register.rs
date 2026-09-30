//! The register of repeats, and a text checked against it (ADR 0044).

use tauri::State;

use super::active;
use crate::actions;
use crate::error::Result;
use crate::register::check::{self, TextCheck};
use crate::register::{self, NewTerm, RegisterEntry, Term, TermPatch, TermUse};
use crate::state::AppState;
use crate::trash::Entity;

/// Every term of the register, with how many works carry it now.
#[tauri::command]
pub fn list_terms(state: State<'_, AppState>) -> Result<Vec<RegisterEntry>> {
    let conn = state.conn();
    register::entries(&conn, &active(&conn)?)
}

/// The works a term is in: found in their current text, or named.
#[tauri::command]
pub fn term_uses(state: State<'_, AppState>, id: String) -> Result<Vec<TermUse>> {
    register::uses(&state.conn(), &id)
}

/// How many works these words are in, before they are a term.
#[tauri::command]
pub fn preview_term(
    state: State<'_, AppState>,
    word: String,
    forms: Option<Vec<String>>,
) -> Result<usize> {
    let conn = state.conn();
    register::preview(&conn, &active(&conn)?, &word, &forms.unwrap_or_default())
}

/// The topics the register groups by, with how many terms each groups.
#[tauri::command]
pub fn list_term_topics(state: State<'_, AppState>) -> Result<Vec<(String, i64)>> {
    let conn = state.conn();
    register::topics(&conn, &active(&conn)?)
}

#[tauri::command]
pub fn create_term(state: State<'_, AppState>, term: NewTerm) -> Result<Term> {
    actions::register::create(&state.conn(), term)
}

#[tauri::command]
pub fn update_term(state: State<'_, AppState>, id: String, patch: TermPatch) -> Result<Term> {
    actions::register::update(&state.conn(), &id, patch)
}

/// Throw a term away - into the trash, with the works it named.
#[tauri::command]
pub fn delete_term(state: State<'_, AppState>, id: String) -> Result<String> {
    actions::trash::discard(&state.conn(), Entity::Term, &id)
}

#[tauri::command]
pub fn link_term(state: State<'_, AppState>, term_id: String, work_id: String) -> Result<()> {
    actions::register::link(&state.conn(), &term_id, &work_id)
}

#[tauri::command]
pub fn unlink_term(state: State<'_, AppState>, term_id: String, work_id: String) -> Result<()> {
    actions::register::unlink(&state.conn(), &term_id, &work_id)
}

/// A text checked against itself and the register: the words it leans on,
/// the terms it takes, and where to mark both.
#[tauri::command]
pub fn check_text(state: State<'_, AppState>, text: String) -> Result<TextCheck> {
    let conn = state.conn();
    check::text(&conn, &active(&conn)?, &text)
}
