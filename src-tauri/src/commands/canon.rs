//! The canon: cards, their facts and relations, their pictures, the timeline,
//! and a card read through the eyes of a task.

use tauri::State;

use super::active;
use crate::actions;
use crate::asset::{Asset, NewAsset};
use crate::canon::view::{self, CardFilter, CardSummary, CardView, Dated};
use crate::canon::{CanonLink, CanonLinkPatch, Fact, FactPatch, NewCanonLink, NewFact};
use crate::error::Result;
use crate::note::{NewNote, Note};
use crate::profile::config::Lens;
use crate::state::AppState;
use crate::trash::Entity;

/// Every card of the profile, the root first, kind by kind.
#[tauri::command]
pub fn list_cards(
    state: State<'_, AppState>,
    filter: Option<CardFilter>,
) -> Result<Vec<CardSummary>> {
    let conn = state.conn();
    view::cards(&conn, &active(&conn)?, &filter.unwrap_or_default())
}

/// One card whole: facts by section with the tasks that read each, relations,
/// pictures, where it appears, whether its description is stale.
#[tauri::command]
pub fn read_card(state: State<'_, AppState>, id: String) -> Result<CardView> {
    view::card(&state.conn(), &id)
}

/// A card as a task's prompt receives it - the text the assistant is handed.
#[tauri::command]
pub fn card_as_seen(state: State<'_, AppState>, id: String, lens: Lens) -> Result<String> {
    view::render(&state.conn(), &id, lens)
}

/// The facts dated inside the world, of one card or of the whole canon.
#[tauri::command]
pub fn canon_timeline(state: State<'_, AppState>, note_id: Option<String>) -> Result<Vec<Dated>> {
    let conn = state.conn();
    view::timeline(&conn, &active(&conn)?, note_id.as_deref())
}

/// A template with its `[[card:id]]` references as a generator reads them.
#[tauri::command]
pub fn expand_card_references(state: State<'_, AppState>, text: String) -> Result<String> {
    view::expand_cards(&state.conn(), &text)
}

#[tauri::command]
pub fn create_card(state: State<'_, AppState>, card: NewNote) -> Result<Note> {
    actions::canon::create_card(&state.conn(), card)
}

/// Write the description a picture generator is given for a card, by hand;
/// `None` clears it.
#[tauri::command]
pub fn describe_card(state: State<'_, AppState>, id: String, text: Option<String>) -> Result<Note> {
    actions::canon::describe(&state.conn(), &id, text, None)
}

#[tauri::command]
pub fn add_fact(state: State<'_, AppState>, fact: NewFact) -> Result<Fact> {
    actions::canon::add_fact(&state.conn(), fact)
}

#[tauri::command]
pub fn update_fact(state: State<'_, AppState>, id: String, patch: FactPatch) -> Result<Fact> {
    actions::canon::update_fact(&state.conn(), &id, patch)
}

#[tauri::command]
pub fn retire_fact(state: State<'_, AppState>, id: String, reason: String) -> Result<Fact> {
    actions::canon::retire_fact(&state.conn(), &id, &reason)
}

#[tauri::command]
pub fn reorder_facts(
    state: State<'_, AppState>,
    note_id: String,
    section: String,
    ids: Vec<String>,
) -> Result<()> {
    actions::canon::reorder_facts(&state.conn(), &note_id, &section, &ids)
}

/// Move a fact to the trash, with its pictures. Returns the trash entry.
#[tauri::command]
pub fn delete_fact(state: State<'_, AppState>, id: String) -> Result<String> {
    actions::trash::discard(&state.conn(), Entity::Fact, &id)
}

#[tauri::command]
pub fn relate_cards(state: State<'_, AppState>, link: NewCanonLink) -> Result<CanonLink> {
    actions::canon::relate(&state.conn(), link)
}

#[tauri::command]
pub fn update_relation(
    state: State<'_, AppState>,
    id: String,
    patch: CanonLinkPatch,
) -> Result<CanonLink> {
    actions::canon::update_relation(&state.conn(), &id, patch)
}

#[tauri::command]
pub fn unrelate_cards(state: State<'_, AppState>, id: String) -> Result<()> {
    actions::canon::unrelate(&state.conn(), &id)
}

/// A picture pasted straight onto a card, or onto one of its facts.
#[tauri::command(async)]
pub fn paste_card_picture(
    state: State<'_, AppState>,
    picture: NewAsset,
    bytes: Vec<u8>,
    name: String,
) -> Result<Asset> {
    let media = state.media_dir()?;
    actions::canon::paste_picture(&state.conn(), &media, picture, &bytes, &name)
}

/// Give a picture of a card another role.
#[tauri::command]
pub fn set_picture_role(state: State<'_, AppState>, id: String, role: String) -> Result<Asset> {
    actions::canon::set_picture_role(&state.conn(), &id, &role)
}

/// Every proposal for the canon nobody has answered, with the cards each
/// touches - what the Canon screen shows beside an open card.
#[tauri::command]
pub fn pending_canon_proposals(
    state: State<'_, AppState>,
) -> Result<Vec<crate::assistant::apply::CanonProposal>> {
    let conn = state.conn();
    crate::assistant::apply::pending_canon(&conn, &active(&conn)?)
}

/// A proposal for the canon read against the canon as it stands now: for each
/// fact, the card and section it lands in, the fact it changes, the facts it
/// contradicts, and whether the card already says the same.
#[tauri::command]
pub fn review_canon_proposal(
    state: State<'_, AppState>,
    message_id: String,
) -> Result<Vec<crate::canon::proposal::FactReview>> {
    let conn = state.conn();
    let message = crate::assistant::message(&conn, &message_id)?
        .ok_or_else(|| crate::Error::not_found("message", &message_id))?;
    let proposal = message
        .meta
        .get("proposal")
        .cloned()
        .map(serde_json::from_value::<crate::assistant::proposal::Proposal>)
        .transpose()?;
    match proposal {
        Some(crate::assistant::proposal::Proposal::Canon { package }) => {
            crate::canon::proposal::review(&conn, &package)
        }
        _ => Ok(Vec::new()),
    }
}
