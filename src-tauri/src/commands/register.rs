//! The register of repeats, and a text checked against it (ADR 0044).

use tauri::State;

use super::active;
use crate::actions;
use crate::error::Result;
use crate::register::block::{self, BlockView, TermBlock};
use crate::register::check::{self, TextCheck};
use crate::register::proposal::WordsPackage;
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
/// the terms it takes, and where to mark both - and, for a text that is
/// `sung`, where its stresses fall (ADR 0053). A version's text names its
/// work and role: a role a composition writes is read against the
/// dictionary instead (v0.94).
#[tauri::command]
pub fn check_text(
    state: State<'_, AppState>,
    text: String,
    sung: Option<bool>,
    work_id: Option<String>,
    role: Option<String>,
) -> Result<TextCheck> {
    let conn = state.conn();
    let profile_id = active(&conn)?;
    let sung = sung.unwrap_or(false);
    match (work_id, role) {
        (Some(work_id), Some(role)) => {
            let work = crate::work::get(&conn, &work_id)?
                .ok_or_else(|| crate::error::Error::not_found("work", work_id))?;
            check::version_text(&conn, &profile_id, &text, sung, &work, &role)
        }
        _ => check::text(&conn, &profile_id, &text, sung),
    }
}

/// A sung text as the public reads it: the stress marks taken off and the
/// owner's respellings put back (ADR 0053).
#[tauri::command]
pub fn clean_text(state: State<'_, AppState>, text: String) -> Result<String> {
    let conn = state.conn();
    let forms = register::sung_forms(&conn, &active(&conn)?)?;
    Ok(crate::words::sung::clean(&text, &forms))
}

/// The blocks of the bank, in the owner's order, each with its words.
#[tauri::command]
pub fn list_blocks(state: State<'_, AppState>) -> Result<Vec<BlockView>> {
    let conn = state.conn();
    block::views(&conn, &active(&conn)?)
}

#[tauri::command]
pub fn create_block(state: State<'_, AppState>, name: String) -> Result<TermBlock> {
    actions::register::create_block(&state.conn(), &name)
}

#[tauri::command]
pub fn rename_block(state: State<'_, AppState>, id: String, name: String) -> Result<TermBlock> {
    actions::register::rename_block(&state.conn(), &id, &name)
}

/// Put a block at `index` in the order; the blocks as they now stand.
#[tauri::command]
pub fn move_block(state: State<'_, AppState>, id: String, index: usize) -> Result<Vec<TermBlock>> {
    actions::register::move_block(&state.conn(), &id, index)
}

/// Throw a block away - into the trash, with the words put in it; the words
/// stay in the bank.
#[tauri::command]
pub fn delete_block(state: State<'_, AppState>, id: String) -> Result<String> {
    actions::trash::discard(&state.conn(), Entity::Block, &id)
}

#[tauri::command]
pub fn add_to_block(state: State<'_, AppState>, block_id: String, term_id: String) -> Result<()> {
    actions::register::add_to_block(&state.conn(), &block_id, &term_id)
}

#[tauri::command]
pub fn remove_from_block(
    state: State<'_, AppState>,
    block_id: String,
    term_id: String,
) -> Result<()> {
    actions::register::remove_from_block(&state.conn(), &block_id, &term_id)
}

/// Put words in the bank - and in a block, when one is named.
#[tauri::command]
pub fn bank_words(
    state: State<'_, AppState>,
    words: Vec<String>,
    block_id: Option<String>,
) -> Result<actions::register::Banked> {
    actions::register::bank_words(&state.conn(), &words, block_id.as_deref())
}

/// The mark every work wears from the guard of repeats on `today`
/// (`YYYY-MM-DD`, the window's own day): a song its own, a publication its
/// song's (ADR 0054).
#[tauri::command]
pub fn repeat_marks(
    state: State<'_, AppState>,
    today: String,
) -> Result<Vec<crate::register::guard::RepeatMark>> {
    let conn = state.conn();
    crate::register::guard::marks(&conn, &active(&conn)?, &today)
}

/// What the guard says about the song a work is, or is made from.
#[tauri::command]
pub fn work_repeats(
    state: State<'_, AppState>,
    work_id: String,
    today: String,
) -> Result<Option<crate::register::guard::Repeats>> {
    crate::register::guard::of_work(&state.conn(), &work_id, &today)
}

/// "I know, I am keeping it" on a word of a song.
#[tauri::command]
pub fn keep_repeat(state: State<'_, AppState>, work_id: String, word: String) -> Result<()> {
    actions::register::keep_repeat(&state.conn(), &work_id, &word)
}

/// Take "I am keeping it" back.
#[tauri::command]
pub fn unkeep_repeat(state: State<'_, AppState>, work_id: String, word: String) -> Result<()> {
    actions::register::unkeep_repeat(&state.conn(), &work_id, &word)
}

/// What the owner's sung texts already say about singing - capital vowels
/// the language pack cannot give, respellings - proposed for the record.
#[tauri::command]
pub fn words_from_texts(state: State<'_, AppState>) -> Result<WordsPackage> {
    let conn = state.conn();
    crate::register::proposal::from_texts(&conn, &active(&conn)?)
}

/// Keep a package of words - whole, or the items named (`word:N`) - in one
/// unit. The ids of the words written.
#[tauri::command]
pub fn keep_words(
    state: State<'_, AppState>,
    package: WordsPackage,
    items: Option<Vec<String>>,
) -> Result<Vec<String>> {
    let conn = state.conn();
    crate::db::unit::atomically(&conn, |conn| {
        crate::register::proposal::keep(conn, package, items.as_deref())
    })
}
